use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rustysynth::{MidiFile, SoundFont};
use tauri::State;

use crate::audio::AudioEngine;
use crate::db;
use crate::midi::{parse_midi, MidiData};

pub struct AppState {
    pub engine: Mutex<AudioEngine>,
    pub db: turso::Connection,
    pub current_sf: Mutex<Option<Arc<SoundFont>>>,
    pub current_midi: Mutex<Option<Arc<MidiFile>>>,
    pub builtin_sf_path: String,
    pub demo_path: String,
}

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn load_sf_arc(path: &str) -> Result<Arc<SoundFont>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let sf = SoundFont::new(&mut std::io::Cursor::new(bytes)).map_err(|e| format!("soundfont load: {e:?}"))?;
    Ok(Arc::new(sf))
}

// Swap the active soundfont and (if a song is loaded) restart it through the new sound.
// ponytail v1: restarts from 0 on soundfont change; carry-position is a later nicety.
fn apply_soundfont(state: &State<'_, AppState>, sf: Arc<SoundFont>) -> Result<(), String> {
    *state.current_sf.lock().map_err(|e| e.to_string())? = Some(sf.clone());
    let midi = state.current_midi.lock().map_err(|e| e.to_string())?.clone();
    if let Some(midi) = midi {
        let dur = midi.get_length();
        state.engine.lock().map_err(|e| e.to_string())?.load(sf, midi, dur);
    }
    Ok(())
}

#[tauri::command]
pub async fn open_midi(state: State<'_, AppState>, path: String) -> Result<MidiData, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let md = parse_midi(&bytes)?;
    let midi = Arc::new(MidiFile::new(&mut std::io::Cursor::new(bytes)).map_err(|e| format!("midi load: {e:?}"))?);

    // Ensure a soundfont is loaded (builtin by default). Lock + drop before any await.
    let sf = {
        let mut cur = state.current_sf.lock().map_err(|e| e.to_string())?;
        if cur.is_none() {
            *cur = Some(load_sf_arc(&state.builtin_sf_path)?);
        }
        cur.as_ref().unwrap().clone()
    };
    { state.engine.lock().map_err(|e| e.to_string())?.load(sf, midi.clone(), md.duration_sec); }
    *state.current_midi.lock().map_err(|e| e.to_string())? = Some(midi);

    db::upsert_recent(&state.db, &path, md.title.as_deref(), md.duration_sec, now_unix()).await?;
    Ok(md)
}

#[tauri::command]
pub fn play(state: State<'_, AppState>) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.play(); Ok(()) }
#[tauri::command]
pub fn pause(state: State<'_, AppState>) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.pause(); Ok(()) }
#[tauri::command]
pub fn seek(state: State<'_, AppState>, seconds: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.seek(seconds); Ok(()) }
#[tauri::command]
pub fn set_tempo(state: State<'_, AppState>, ratio: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.set_speed(ratio); Ok(()) }
#[tauri::command]
pub fn set_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> { state.engine.lock().map_err(|e| e.to_string())?.set_volume(volume as f32); Ok(()) }

#[tauri::command]
pub async fn load_soundfont(state: State<'_, AppState>, path: String) -> Result<db::SoundfontRow, String> {
    let sf = load_sf_arc(&path)?;
    apply_soundfont(&state, sf)?;
    let name = std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("SoundFont").to_string();
    db::register_soundfont(&state.db, &path, &name, false).await
}

#[tauri::command]
pub async fn set_soundfont(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let row = db::list_soundfonts(&state.db).await?.into_iter().find(|s| s.id == id).ok_or("unknown soundfont id")?;
    let sf = load_sf_arc(&row.path)?;
    apply_soundfont(&state, sf)
}

#[tauri::command]
pub async fn list_soundfonts(state: State<'_, AppState>) -> Result<Vec<db::SoundfontRow>, String> { db::list_soundfonts(&state.db).await }
#[tauri::command]
pub async fn list_recent(state: State<'_, AppState>) -> Result<Vec<db::LibraryRow>, String> { db::list_recent(&state.db, 20).await }
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Vec<db::Setting>, String> { db::list_settings(&state.db).await }
#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> { db::set_setting(&state.db, &key, &value).await }

#[tauri::command]
pub fn demo_path(state: State<'_, AppState>) -> Result<String, String> { Ok(state.demo_path.clone()) }
