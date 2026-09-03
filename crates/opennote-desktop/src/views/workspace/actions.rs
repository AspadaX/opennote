use std::io::Read;

use gpui::*;
use gpui_component::Root;

use opennote_data::Databases;
use opennote_embedder::entry::EmbedderEntry;
use opennote_models::configurations::fields::VectorDatabaseConfig;

use crate::{
    globals::{
        actions::{block::build_block, route_helpers::route_create_blocks},
        bootstrap::GlobalApplicationBootStrap,
        helpers::{get_language_profile, run_async_background},
        states::{States, helpers::get_states, server_registry::ServerStates},
        tasks::{
            task_information::TaskInformation,
            task_result::{TaskResult, TaskType},
            tracker::{register_long_running_completion, register_long_running_task},
            unique_notifications::ImportNBlocksNotification,
        },
    },
    key_mappings::mappings::{
        CloseActiveTab, CreateOneBlock, ImportFiles, NextTab, OpenNewWindow, PreviousTab,
        ToggleCommandBar, ToggleSearchBar, ToggleSettingsPanel, ToggleSidebar,
    },
};

use super::Workspace;

impl Workspace {
    /// Toggle the sidebar visibility and shift focus accordingly.
    pub fn toggle_sidebar(
        &mut self,
        _action: &ToggleSidebar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            this.toggle(cx);

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled() {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled() {
                let states = get_states(cx);
                let active_server =
                    states.get_active_server_name(window.window_handle().window_id());
                if let Some(tree_state) = this.get_tree_focus_handle(cx, &active_server) {
                    window.focus(&tree_state);
                }
            }
        });

        cx.notify();
    }

    /// Toggle the search bar and shift focus accordingly.
    pub fn toggle_search_bar(
        &mut self,
        _action: &ToggleSearchBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                let mut selected_text = None;

                let _ = this.editor.update(cx, |this, cx| {
                    let _ = this.state.update(cx, |this, cx| {
                        selected_text = this.selected_markdown_text(cx);
                    });
                });

                if let Some(query) = selected_text {
                    // Keeping newlines will cause gpui to panic in single-line rendering mode,
                    // when rendering the search input box
                    let query: String = query.lines().map(|item| item.replace("\n", " ")).collect();

                    this.search_results_list.update(cx, |this, cx| {
                        this.update_query_input_mut(cx, window, query);
                    });
                }

                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Toggle the command bar and shift focus accordingly.
    pub fn toggle_command_bar(
        &mut self,
        _action: &ToggleCommandBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_bar.update(cx, |this, cx| {
            this.is_toggled = !this.is_toggled;

            // Manually shift the focus, otherwise it won't just focus automatically
            if !this.is_toggled {
                window.focus(&self.focus_handle(cx));
            }

            if this.is_toggled {
                window.focus(&this.get_input_field_focus_handle(cx));
            }
        });

        cx.notify();
    }

    /// Create a new block in the active server's tree.
    pub fn create_one_block(
        &mut self,
        _action: &CreateOneBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.update(cx, |this, cx| {
            let states = get_states(cx);
            let active_server = states.get_active_server_name(window.window_handle().window_id());
            let tree_state = this.get_tree_state(&active_server);

            if let Some(tree_state) = tree_state {
                this.handle_block_creation(window, cx, tree_state);
            }
        })
    }

    /// Open the settings panel in a new window.
    pub fn toggle_settings_panel(
        &mut self,
        _action: &ToggleSettingsPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings_panel = self.settings_panel.clone();
        let _ = cx
            .open_window(WindowOptions::default(), |_this, cx| {
                cx.new(|cx| Root::new(settings_panel, window, cx))
            })
            .unwrap();
    }

    /// Switch to the next tab in the active pane.
    pub fn next_tab(&mut self, _action: &NextTab, _window: &mut Window, cx: &mut Context<Self>) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_next_tab(cx);
        });
    }

    /// Switch to the previous tab in the active pane.
    pub fn previous_tab(
        &mut self,
        _action: &PreviousTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            this.activate_previous_tab(cx);
        });
    }

    /// Open a new workspace window.
    pub fn open_new_window(
        &mut self,
        _action: &OpenNewWindow,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.open_window(WindowOptions::default(), |window, cx| {
            let view = cx.new(|cx| {
                let workspace =
                    Workspace::new(window, cx).expect("Workspace initialization failed");
                workspace
            });

            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("Failed to open window");
    }

    /// Close the active tab in the active pane.
    pub fn close_active_tab(
        &mut self,
        _action: &CloseActiveTab,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let states = get_states(cx);
        let Some(active_pane) = states.get_active_pane(cx) else {
            return;
        };

        let _ = active_pane.update(cx, |this, cx| {
            if let Some(selected_block_id) = this.selected_block_id {
                this.close_tab(&selected_block_id, cx);
            }
        });
    }

    pub fn import_files(
        &mut self,
        _action: &ImportFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let language_profile = get_language_profile(cx).unwrap();
        let importing_message = language_profile["importing_n_blocks"].clone();
        let imported_message = language_profile["imported_n_blocks"].clone();
        let import_failed_message = language_profile["block_import_failed"].clone();

        // Open a dialogue to pick files
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });

        let window = window.window_handle();

        cx.spawn(async move |_this, cx| {
            let paths = match prompt.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(_error)) => return,
            };

            let num_blocks = paths.len();
            let task = TaskInformation::new(
                importing_message.replace("{}", &num_blocks.to_string()),
                TaskType::ImportNBlocks,
                true,
            );

            let task_id = task.id;

            // Register task in the scheduler.
            register_long_running_task::<ImportNBlocksNotification>(window, cx, task);

            let (databases, embedders, document_chunk_size, vector_database_config) = cx
                .read_global::<GlobalApplicationBootStrap, (Databases, EmbedderEntry, usize, VectorDatabaseConfig)>(
                    |this, _cx| {
                        let configurations = this.get_configurations();

                        (
                            this.0.databases.clone(),
                            this.0.embedders.clone(),
                            configurations.user.search.document_chunk_size,
                            configurations.system.vector_database.clone(),
                        )
                    },
                )
                .unwrap();

            let (server_name, server_states) = cx
                .read_global::<States, (SharedString, ServerStates)>(|this, _cx| {
                    this.get_active_server(window.window_id())
                })
                .unwrap();

            let executor = cx.background_executor();
            let tokio_handle = tokio::runtime::Handle::current();

            let mut results = Vec::new();

            for path in paths {
                let embedders = embedders.clone();

                let Some(raw_file_name) = path.file_name() else {
                    continue;
                };

                let raw_file_name = raw_file_name.to_string_lossy().to_string();

                let mut content = String::new();

                match std::fs::File::open(&path) {
                    Ok(mut file) => {
                        file.read_to_string(&mut content).unwrap();
                    }
                    Err(_error) => return,
                };

                // Create blocks for files
                let result = run_async_background(
                    executor, tokio_handle.clone(), async move {
                        build_block(
                            None,
                            raw_file_name,
                            &embedders,
                            Some(content),
                            Some(document_chunk_size),
                        ).await
                    }
                ).await;
                
                results.push(result);
            }

            let mut blocks = Vec::new();
            for block in results {
                match block {
                    Ok(result) => {
                        blocks.push(result);
                    },
                    Err(error) => {
                        log::error!("Failed to build imported block: {}", error);
                        register_long_running_completion::<ImportNBlocksNotification>(
                            window,
                            cx,
                            TaskResult::new(
                                task_id,
                                false,
                                import_failed_message.replace("{}", &error.to_string()),
                                TaskType::ImportNBlocks,
                                None,
                            ),
                        );
                        return;
                    }
                }
            }

            // Store the blocks to the active server
            match route_create_blocks(
                &server_name,
                &server_states,
                &databases,
                &vector_database_config,
                blocks,
            )
            .await
            {
                Ok(_) => {}
                Err(error) => {
                    log::error!("Failed to store imported blocks: {}", error);
                    register_long_running_completion::<ImportNBlocksNotification>(
                        window,
                        cx,
                        TaskResult::new(
                            task_id,
                            false,
                            import_failed_message.replace("{}", &error.to_string()),
                            TaskType::ImportNBlocks,
                            None,
                        ),
                    );
                    return;
                }
            }

            register_long_running_completion::<ImportNBlocksNotification>(
                window,
                cx,
                TaskResult::new(
                    task_id,
                    true,
                    imported_message.replace("{}", &num_blocks.to_string()),
                    TaskType::ImportNBlocks,
                    None,
                ),
            );

            // Refresh the sidebar
            let _ = cx.update_global::<States, ()>(|this, cx| {
                this.refresh_blocks_list(cx);
            });
        }).detach();
    }
}
