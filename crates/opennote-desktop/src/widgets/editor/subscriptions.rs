use gpui::{Context, Entity};

use opennote_velotype::editor::EditorEvent;

use crate::widgets::editor::Editor;

pub fn subscribe_editor_events(
    view: &mut Editor,
    _state: &Entity<opennote_velotype::editor::Editor>,
    event: &EditorEvent,
    window: &mut gpui::Window,
    cx: &mut Context<'_, Editor>,
) {
    let pane_clone = view.pane.clone();

    match event {
        EditorEvent::ContentChanged => {
            let Some(block) = &view.block else {
                return;
            };

            let _ = pane_clone.update(cx, |this, _cx| {
                this.opened_tab_states
                    .update_tab_save_state(window, &block.id, false);
            });
        }
    }
}
