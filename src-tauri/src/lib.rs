mod analysis;
mod audio;
mod commands;
mod db;
mod midi;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve the bundled soundfont (Tauri resource).
            let sf_path = app
                .path()
                .resolve("soundfonts/GeneralUser-GS.sf2", tauri::path::BaseDirectory::Resource)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();

            // Resolve the bundled demo MIDI (Tauri resource).
            let demo_path = app
                .path()
                .resolve("demo/scale.mid", tauri::path::BaseDirectory::Resource)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned();

            // Open the local Turso cache in the app data dir (one-time blocking init).
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).ok();
            let db_file = data_dir.join("midimi.db").to_string_lossy().into_owned();
            let conn = tauri::async_runtime::block_on(async { db::open(&db_file).await })
                .map_err(|e| e.to_string())?;

            // Register the builtin soundfont (best effort).
            {
                let conn = conn.clone();
                let p = sf_path.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = db::register_soundfont(&conn, &p, "GeneralUser GS", true).await;
                });
            }

            // Audio engine + its shared snapshot.
            let engine = audio::AudioEngine::new().map_err(|e| e.to_string())?;
            let shared = engine.shared();

            app.manage(AppState {
                engine: Mutex::new(engine),
                db: conn,
                current_sf: Mutex::new(None),
                current_midi: Mutex::new(None),
                builtin_sf_path: sf_path,
                demo_path,
            });

            // 60 Hz playhead emit loop.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let snap = shared.lock().ok().map(|s| s.clone());
                if let Some(s) = snap {
                    let _ = handle.emit(
                        "playhead",
                        serde_json::json!({
                            "time": s.position_sec,
                            "duration": s.duration_sec,
                            "level": s.level,
                            "bands": s.bands,
                            "playing": s.playing,
                        }),
                    );
                }
                std::thread::sleep(Duration::from_millis(16));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_midi,
            commands::play,
            commands::pause,
            commands::seek,
            commands::set_tempo,
            commands::set_volume,
            commands::load_soundfont,
            commands::set_soundfont,
            commands::list_soundfonts,
            commands::list_recent,
            commands::get_settings,
            commands::set_setting,
            commands::demo_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running midimi");
}
