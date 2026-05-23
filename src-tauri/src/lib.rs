pub mod commands;
pub mod errors;
pub mod models;
pub mod services;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::AppState::default())
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
            commands::chat_ask,
            commands::chat_ask_stream,
            commands::chat_build_embeddings,
            commands::get_chat_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
