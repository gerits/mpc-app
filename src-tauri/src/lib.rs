pub mod assets;
pub mod audio;
pub mod midi;
pub mod router;
pub mod soundfont_manager;
pub mod synth;
pub mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
