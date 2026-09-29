mod application;
mod commands;
mod domain;
mod infrastructure;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::library::import_pdf,
            commands::library::list_papers,
            commands::library::update_metadata,
            commands::reader::open_document,
            commands::reader::save_reading_position,
            commands::reader::list_annotations,
            commands::reader::create_annotation,
            commands::reader::delete_annotation,
            commands::conversion::get_markdown_document,
            commands::conversion::get_mineru_settings,
            commands::conversion::save_mineru_settings,
            commands::conversion::convert_pdf_to_markdown,
            commands::translation::get_translation_settings,
            commands::translation::save_translation_settings,
            commands::translation::translate_text
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let database_path = data_dir.join("reflo.sqlite");
            infrastructure::db::initialize(&database_path)?;

            println!("数据库已就绪：{}", database_path.display());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Reflo 失败");
}
