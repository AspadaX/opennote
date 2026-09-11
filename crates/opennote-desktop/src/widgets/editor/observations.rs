use gpui::{BorrowAppContext, Context};
use gpui_component::{ThemeMode, WindowExt};

use opennote_models::block::Block;

use crate::{
    globals::{
        actions::update_n_blocks,
        states::helpers::get_states,
        tasks::{
            task_result::{TaskResult, TaskType},
            tracker::TaskTracker,
            unique_notifications::ChunkBlockNotification,
        },
    },
    widgets::editor::Editor,
};

pub fn observe_theme_change(
    this: &mut Editor,
    window: &mut gpui::Window,
    cx: &mut Context<'_, Editor>,
) {
    let theme_mode = ThemeMode::from(window.appearance());

    let switch_to_dark_mode = match theme_mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
    };

    this.state.update(cx, |_this, cx| {
        opennote_velotype::editor::Editor::switch_theme(cx, switch_to_dark_mode);
    });
}

pub fn observe_chunk_block(
    this: &mut Editor,
    window: &mut gpui::Window,
    cx: &mut Context<'_, Editor>,
) {
    let Some(active_window) = cx.active_window() else {
        return;
    };

    let pane_clone = this.pane.clone();

    let active_window_id = active_window.window_id();

    // Global observers run for every window. Only the active window may consume
    // results from its tracker group.
    if window.window_handle().window_id() != active_window_id {
        return;
    }

    let Some(block) = &this.block else {
        return;
    };

    let task_type = TaskType::ChunkBlock { block_id: block.id };
    let scheduler: &TaskTracker = cx.global();
    if !scheduler.has_pending_task_results(active_window_id, Some(task_type)) {
        return;
    }

    let task_result = cx.update_global::<TaskTracker, Option<TaskResult>>(|this, _cx| {
        this.get_task_result(active_window_id, task_type)
    });

    if let Some(result) = task_result {
        window.remove_notification::<ChunkBlockNotification>(cx);

        let block: Block = if let Some(data) = result.data {
            serde_json::from_value(data).unwrap()
        } else {
            return;
        };

        let states = get_states(cx);
        let servers = states.get_servers_by_block_ids(&vec![block.id]).remove(0);

        update_n_blocks(window, cx, vec![block], servers.0, servers.1, true);
    }

    // Alter the tab's save state to true
    let _ = pane_clone.update(cx, |this, _cx| {
        this.opened_tab_states
            .update_tab_save_state(window, &block.id, true);
    });
}
