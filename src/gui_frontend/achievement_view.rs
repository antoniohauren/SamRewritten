// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::gui_frontend::MainApplication;
use crate::gui_frontend::achievement_automatic_view::create_achievements_automatic_view;
use crate::gui_frontend::achievement_manual_view::create_achievements_manual_view;
use crate::gui_frontend::gobjects::achievement::GAchievementObject;
use crate::gui_frontend::i18n::tr;
use gtk::gio::ListStore;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Box, CustomFilter, CustomSorter, FilterChange, FilterListModel, Label, NoSelection,
    Orientation, SortListModel, Stack, StackTransitionType, StringFilter, StringFilterMatchMode,
    ToggleButton,
};
use std::cell::Cell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn create_achievements_view(
    app_id: Rc<Cell<Option<u32>>>,
    app_unlocked_achievements_count: Rc<Cell<usize>>,
    application: &MainApplication,
    app_achievement_count_value: &Label,
) -> (Stack, ListStore, StringFilter, Arc<AtomicBool>, Box) {
    let app_achievements_model = ListStore::new::<GAchievementObject>();
    let app_timed_achievements_model = ListStore::new::<GAchievementObject>();

    let app_achievement_string_filter = StringFilter::builder()
        .expression(GAchievementObject::this_expression("search-text"))
        .match_mode(StringFilterMatchMode::Substring)
        .ignore_case(true)
        .build();
    let app_achievement_filter_model = FilterListModel::builder()
        .model(&app_achievements_model)
        .filter(&app_achievement_string_filter)
        .build();
    // None shows all achievements; Some(false) and Some(true) show only
    // locked and unlocked achievements respectively.
    let achievement_status = Rc::new(Cell::new(None));
    let achievement_status_filter = CustomFilter::new({
        let achievement_status = Rc::clone(&achievement_status);
        move |object| {
            let achievement = object.downcast_ref::<GAchievementObject>().unwrap();
            achievement_status
                .get()
                .is_none_or(|unlocked| achievement.is_achieved() == unlocked)
        }
    });
    let app_achievement_status_filter_model = FilterListModel::builder()
        .model(&app_achievement_filter_model)
        .filter(&achievement_status_filter)
        .build();
    let all_toggle = ToggleButton::builder()
        .icon_name("object-select-symbolic")
        .tooltip_text(tr("All").as_str())
        .build();
    let locked_toggle = ToggleButton::builder()
        .icon_name("changes-prevent-symbolic")
        .tooltip_text(tr("Locked").as_str())
        .group(&all_toggle)
        .build();
    let unlocked_toggle = ToggleButton::builder()
        .icon_name("changes-allow-symbolic")
        .tooltip_text(tr("Unlocked").as_str())
        .group(&all_toggle)
        .build();
    all_toggle.set_active(true);
    let status_filter_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(["linked"])
        .valign(Align::Center)
        .visible(false)
        .build();
    status_filter_box.append(&all_toggle);
    status_filter_box.append(&locked_toggle);
    status_filter_box.append(&unlocked_toggle);
    for (button, state) in [
        (&all_toggle, None),
        (&locked_toggle, Some(false)),
        (&unlocked_toggle, Some(true)),
    ] {
        button.connect_toggled(glib::clone!(
            #[strong]
            achievement_status,
            #[weak]
            achievement_status_filter,
            move |button| {
                if button.is_active() {
                    achievement_status.set(state);
                    achievement_status_filter.changed(FilterChange::Different);
                }
            }
        ));
    }
    let app_achievement_timed_filter_model = FilterListModel::builder()
        .model(&app_timed_achievements_model)
        .filter(&app_achievement_string_filter)
        .build();

    let global_achieved_percent_sorter = CustomSorter::new(move |obj1, obj2| {
        let achievement1 = obj1.downcast_ref::<GAchievementObject>().unwrap();
        let achievement2 = obj2.downcast_ref::<GAchievementObject>().unwrap();

        let percent1 = achievement1.global_achieved_percent();
        let percent2 = achievement2.global_achieved_percent();

        percent2
            .partial_cmp(&percent1)
            .unwrap_or(Ordering::Equal)
            .into()
    });
    let app_achievement_sort_model = SortListModel::builder()
        .model(&app_achievement_status_filter_model)
        .sorter(&global_achieved_percent_sorter)
        .build();

    let app_achievement_selection_model = NoSelection::new(Option::<ListStore>::None);
    app_achievement_selection_model.set_model(Some(&app_achievement_sort_model));
    let app_timed_achievement_selection_model = NoSelection::new(Option::<ListStore>::None);
    app_timed_achievement_selection_model.set_model(Some(&app_achievement_timed_filter_model));

    let achievement_views_stack = Stack::builder()
        .transition_type(StackTransitionType::SlideLeftRight)
        .build();
    let (achievements_manual_frame, cancel_timed_unlock) = create_achievements_manual_view(
        &app_id,
        &app_unlocked_achievements_count,
        &app_achievement_selection_model,
        &achievement_status_filter,
        &app_achievements_model,
        &app_timed_achievements_model,
        &achievement_views_stack,
        app_achievement_count_value,
        application,
    );
    let (achievements_automatic_frame, _achievements_automatic_stop) =
        create_achievements_automatic_view(&app_timed_achievement_selection_model, application);

    achievement_views_stack.add_named(&achievements_manual_frame, Some("manual"));
    achievement_views_stack.add_named(&achievements_automatic_frame, Some("automatic"));

    (
        achievement_views_stack,
        app_achievements_model,
        app_achievement_string_filter,
        cancel_timed_unlock,
        status_filter_box,
    )
}
