use std::collections::HashMap;

use gpui::{Context, Window, prelude::*};
use gpui::{ElementId, SharedString, WeakEntity};
use gpui_component::button::{Button, ButtonRounded, ButtonVariants};
use gpui_component::{IconName, Selectable, Sizable};
use uuid::Uuid;

use crate::globals::states::helpers::get_states;
use crate::libs::tabs::drag::DraggedItem;
use crate::libs::tabs::tab::Tab;
use crate::libs::tabs::tab_bar::TabBar;
use crate::widgets::pane::Pane;

pub struct TabState {
    /// It is saved when a document has just opened.
    ///
    /// Once a text change has detected, this becomes false.
    ///
    /// Once a SaveDocument action has been successfully completed,
    /// this becomes true
    pub has_saved: bool,

    pub unsaved_content: Option<SharedString>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            has_saved: true,
            unsaved_content: None,
        }
    }
}

/// Key: block_id
/// Value: TabState
pub struct TabStates(HashMap<Uuid, TabState>);

impl TabStates {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn does_tab_exist(&self, block_id: &Uuid) -> bool {
        self.0.contains_key(block_id)
    }

    /// This will return a false when the tab does not exist.
    pub fn has_tab_saved(&self, block_id: &Uuid) -> bool {
        match self.0.get(block_id) {
            Some(result) => result.has_saved,
            None => false,
        }
    }

    pub fn create_tab_state(&mut self, block_id: &Uuid) {
        self.0.insert(
            *block_id,
            TabState {
                ..Default::default()
            },
        );
    }

    pub fn update_tab_save_state(&mut self, window: &mut Window, block_id: &Uuid, has_saved: bool) {
        if let Some(tab_state) = self.0.get_mut(block_id) {
            tab_state.has_saved = has_saved;

            // As long as there's one tab remains edited,
            // the corresponding window should also remain edited.
            if !tab_state.has_saved {
                window.set_window_edited(true);
            }
        }

        self.cleanup_window_edited_state(window);
    }

    /// Check if the window is good to remove the edited state.
    fn cleanup_window_edited_state(&self, window: &mut Window) {
        let has_unsaved_contents = self
            .0
            .iter()
            .any(|(_block_id, tab_state)| !tab_state.has_saved);

        if !has_unsaved_contents {
            window.set_window_edited(false);
        }
    }

    pub fn store_unsaved_content(&mut self, block_id: &Uuid, unsaved: SharedString) {
        if let Some(tab_state) = self.0.get_mut(&block_id) {
            tab_state.unsaved_content = Some(unsaved);
        }
    }

    pub fn take_tab_content(&mut self, block_id: &Uuid) -> Option<SharedString> {
        if let Some(tab_state) = self.0.get_mut(block_id) {
            return tab_state.unsaved_content.take();
        }

        None
    }

    pub fn remove_tab_state(&mut self, block_id: &Uuid, window: &mut Window) {
        self.0.remove(block_id);
        self.cleanup_window_edited_state(window);
    }

    pub fn remove_all_tab_state(&mut self, window: &mut Window) {
        self.0.clear();
        self.cleanup_window_edited_state(window);
    }
}

pub fn create_tab_bar_for_blocks(
    cx: &mut Context<'_, Pane>,
    pane_reference: WeakEntity<Pane>,
    pane_id: Uuid,
    opened_block_ids: &Vec<Uuid>,
    selected_block_id: Option<Uuid>,
    openned_tab_states: &TabStates,
) -> TabBar {
    let tabs = TabBar::new("tabs").children(opened_block_ids.iter().map(|id| {
        let id = id.clone();
        let mut selected = false;

        // If we can't get the tab state, that means the application is not synced.
        // Then we probably need to quit the app.
        if !openned_tab_states.does_tab_exist(&id) {
            panic!("Opened blocks' states dis-synced. Aborted")
        }

        // The active block is the focused block
        if let Some(selected_block_id) = &selected_block_id {
            if *selected_block_id == id {
                selected = true;
            }
        }

        // Get the title of the block
        let states = get_states(cx);
        let mut title = String::new();
        if let Some(block) = states.get_block(&id) {
            title = block.get_title();
        }

        // Construct the item for dragging
        let dragged_item = DraggedItem {
            label: Some(SharedString::from(title.clone())),
            owner_pane: Some(pane_reference.clone()),
            owner_pane_id: Some(pane_id),
            block_id: Some(id),
            ..Default::default()
        };

        let mut tab = Tab::new()
            .label(title)
            .selected(selected)
            .suffix(
                Button::new(ElementId::Name(SharedString::from(format!("close-{}", id))))
                    .icon(IconName::CircleX)
                    .ghost()
                    .xsmall()
                    .rounded(ButtonRounded::Medium)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.close_tab(&id, cx, window);
                        cx.stop_propagation();
                    })),
            )
            .on_click(
                cx.listener(move |view, event: &gpui::ClickEvent, _window, _cx| {
                    if !event.is_right_click() {
                        view.selected_block_id = Some(id)
                    }
                }),
            )
            .on_drag(
                dragged_item.clone(),
                move |value: &DraggedItem, _point, _window, app| app.new(|_| value.clone()),
            );

        if !openned_tab_states.has_tab_saved(&id) {
            tab = tab.prefix("⏺");
        }

        tab
    }));

    tabs
}
