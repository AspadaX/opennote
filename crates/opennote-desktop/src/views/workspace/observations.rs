use gpui::{Context, Entity, Window};

use crate::{
    globals::states::helpers::get_states, views::workspace::Workspace, widgets::pane::Pane,
    window::format_window_title,
};

pub fn observe_pane_for_updating_window_title(
    _workspace: &mut Workspace,
    this: Entity<Pane>,
    window: &mut Window,
    cx: &mut Context<'_, Workspace>,
) {
    // recompute the window title
    let states = get_states(cx);

    let pane = this.read(cx);

    // TODO: Get the selected_block_id from the pane instead, not the editor.
    // Maybe add an annotation for standardized access
    let document_name = match &pane.selected_block_id {
        Some(block_id) => Some(states.get_block(block_id).unwrap().get_title()),
        None => None,
    };

    let server_name = match pane.selected_block_id {
        Some(block_id) => Some(
            states.get_servers_by_block_ids(&vec![block_id])[0]
                .0
                .to_string(),
        ),
        None => None,
    };

    window.set_window_title(&format_window_title(
        None,
        server_name.as_deref(),
        document_name.as_deref(),
    ));
}
