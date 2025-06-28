// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, Emitter, Manager, State, Window};

#[tauri::command]
fn app_control(window: Window, cmd: &str) {
    if let Some(command) = cmd.strip_prefix("set_window_size:") {
        if let Ok(size) = serde_json::from_str::<serde_json::Value>(command) {
            if let (Some(width), Some(height)) = (size["width"].as_u64(), size["height"].as_u64()) {
                let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                    width: width as u32,
                    height: height as u32,
                }));
            }
        }
    }
}

use log::{error, info};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

struct MidiState {
    in_connection: Mutex<Option<MidiInputConnection<()>>>,
    out_connection: Mutex<Option<MidiOutputConnection>>,
}

impl Default for MidiState {
    fn default() -> Self {
        Self {
            in_connection: Mutex::new(None),
            out_connection: Mutex::new(None),
        }
    }
}

#[tauri::command]
fn fs_separator() -> String {
    std::path::MAIN_SEPARATOR.to_string()
}

const DIRECTORY_NAME: &str = "BOSS TONE STUDIO for KATANA Gen 3";

fn temporary_path() -> PathBuf {
    let mut temporary_path = dirs::cache_dir().unwrap();
    temporary_path.push(DIRECTORY_NAME);
    temporary_path
}

fn library_path() -> PathBuf {
    let mut library_path = dirs::data_dir().unwrap();
    library_path.push(DIRECTORY_NAME);
    library_path
}

#[tauri::command]
fn fs_path(where_str: &str) -> Result<String, String> {
    let path = match where_str {
        "temporary" => Some(temporary_path()),
        "library" => Some(library_path()),
        _ => None,
    }
    .ok_or_else(|| format!("Unsupported path: {}", where_str))?;
    let mut s = dunce::canonicalize(path)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or_else(|| "Invalid UTF-8 in path".to_string())
        .map(|s| s.to_string())
        .unwrap();
    s.push('/');
    Ok(s)
}

#[tauri::command]
async fn http_request(options: serde_json::Value) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = options["url"].as_str().ok_or("URL not provided")?;
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    response.text().await.map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_mkdir(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_contents(path: String) -> Result<Vec<String>, String> {
    let mut contents = vec![];
    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    contents.push(entry.file_name().to_string_lossy().to_string());
                }
            }
            Ok(contents)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn fs_read_string(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_write_string(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_copy(from: &str, to: &str) -> Result<(), String> {
    std::fs::copy(from, to)
        .map_err(|e| e.to_string())
        .map(|_| ())
}

#[tauri::command]
fn fs_unlink(path: &str) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn fs_volumes() -> String {
    "[]".to_string()
}

/// Represents a MIDI endpoint object passed as an argument to
/// `native.midi.input.connect` and `native.midi.output.connect`.
///
/// The structure is identical to the object returned by the `endpoints` functions.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BossMidiEndpoint {
    // #[serde(rename = "MIDIEndpointUIDKey")]
    // pub midi_endpoint_uid_key: String,
    #[serde(rename = "MIDIEntityNameKey")]
    pub midi_entity_name_key: String,
}

#[tauri::command]
fn midi_inendpoints() -> Result<Vec<BossMidiEndpoint>, String> {
    let midi_in = MidiInput::new("BOSS TONE STUDIO for KATANA Gen 3").map_err(|e| e.to_string())?;
    let endpoints: Vec<BossMidiEndpoint> = midi_in
        .ports()
        .iter()
        .map(|p| {
            let name = midi_in
                .port_name(p)
                .unwrap_or_else(|_| "Unknown".to_string());
            BossMidiEndpoint {
                midi_entity_name_key: name,
            }
        })
        .collect();
    Ok(endpoints)
}

#[tauri::command]
fn midi_outendpoints() -> Result<Vec<BossMidiEndpoint>, String> {
    let midi_out =
        MidiOutput::new("BOSS TONE STUDIO for KATANA Gen 3").map_err(|e| e.to_string())?;
    let endpoints: Vec<BossMidiEndpoint> = midi_out
        .ports()
        .iter()
        .map(|p| {
            let name = midi_out
                .port_name(p)
                .unwrap_or_else(|_| "Unknown".to_string());
            BossMidiEndpoint {
                midi_entity_name_key: name,
            }
        })
        .collect();
    Ok(endpoints)
}

#[tauri::command]
fn midi_inconnect(
    state: State<MidiState>,
    app: AppHandle,
    endpoint: BossMidiEndpoint,
) -> Result<(), String> {
    let midi_in = MidiInput::new("BOSS TONE STUDIO for KATANA Gen 3").map_err(|e| e.to_string())?;
    let port = midi_in
        .ports()
        .iter()
        .find(|p| {
            midi_in.port_name(p).as_deref().unwrap_or_default() == endpoint.midi_entity_name_key
        })
        .cloned()
        .ok_or(format!(
            "Port '{}' not found",
            endpoint.midi_entity_name_key
        ))?;
    let mut conn_lock = state.in_connection.lock().map_err(|e| e.to_string())?;
    let conn = midi_in
        .connect(
            &port,
            "bts-in",
            move |stamp, message, _| {
                let _ = app.emit("midi_message", (hex::encode_upper(message), stamp));
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    *conn_lock = Some(conn);
    Ok(())
}

#[tauri::command]
fn midi_outconnect(state: State<MidiState>, endpoint: BossMidiEndpoint) -> Result<(), String> {
    let midi_out =
        MidiOutput::new("BOSS TONE STUDIO for KATANA Gen 3").map_err(|e| e.to_string())?;
    let port = midi_out
        .ports()
        .iter()
        .find(|p| {
            midi_out.port_name(p).as_deref().unwrap_or_default() == endpoint.midi_entity_name_key
        })
        .cloned()
        .ok_or(format!(
            "Port '{}' not found",
            endpoint.midi_entity_name_key
        ))?;
    let mut conn_lock = state.out_connection.lock().map_err(|e| e.to_string())?;
    let conn = midi_out
        .connect(&port, "bts-out")
        .map_err(|e| e.to_string())?;
    *conn_lock = Some(conn);
    Ok(())
}

#[tauri::command]
fn midi_send(state: State<MidiState>, msg: String) -> Result<(), String> {
    let decoded = hex::decode(msg).map_err(|e| e.to_string())?;
    let mut conn_lock = state.out_connection.lock().map_err(|e| e.to_string())?;
    if let Some(conn) = conn_lock.as_mut() {
        conn.send(&decoded).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    std::fs::create_dir_all(temporary_path()).unwrap();
    std::fs::create_dir_all(library_path()).unwrap();

    tauri::Builder::default()
        .setup(|app| {
            app.manage(MidiState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_control,
            fs_separator,
            fs_path,
            fs_mkdir,
            fs_contents,
            fs_read_string,
            fs_write_string,
            fs_copy,
            fs_unlink,
            fs_volumes,
            http_request,
            midi_inendpoints,
            midi_outendpoints,
            midi_inconnect,
            midi_outconnect,
            midi_send
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
