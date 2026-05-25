pub mod commands;
pub mod errors;
pub mod http_server;
pub mod models;
pub mod services;
pub mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::AppState::default())
        .setup(|app| {
            let state = app.state::<state::AppState>();
            let settings = state.ai_settings.lock().unwrap().clone();
            if settings.http_api_enabled {
                let shared = std::sync::Arc::new(state::AppState::default());
                *shared.ai_settings.lock().unwrap() = settings.clone();
                match http_server::start_http_server(shared, settings.http_api_port) {
                    Ok(handle) => {
                        *state.http_server_handle.lock().unwrap() = Some(handle);
                    }
                    Err(e) => {
                        eprintln!("Failed to start HTTP API server: {}", e);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::open_project,
            commands::save_project,
            commands::get_recent_project,
            commands::set_recent_project,
            commands::get_default_project_dir,
            commands::import_pdfs,
            commands::update_paper,
            commands::delete_paper,
            commands::extract_pdf_text,
            commands::get_pdf_file_url,
            commands::read_pdf_file,
            commands::resolve_metadata,
            commands::search_metadata_candidates,
            commands::apply_metadata_candidate,
            commands::add_relation,
            commands::update_relation,
            commands::delete_relation,
            commands::save_graph_layout,
            commands::run_insight_analysis,
            commands::load_saved_insights,
            commands::save_insights,
            commands::ai_parse_pdf,
            commands::ai_recommend_relations,
            commands::ai_generate_insights,
            commands::test_ai_connection,
            commands::test_ai_connection_stored,
            commands::save_ai_settings,
            commands::get_ai_settings_masked,
            commands::toggle_http_server,
            commands::get_http_server_status,
            commands::chat_ask,
            commands::chat_ask_stream,
            commands::chat_build_embeddings,
            commands::get_chat_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
