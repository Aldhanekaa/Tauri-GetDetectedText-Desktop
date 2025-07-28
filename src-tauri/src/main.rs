// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri_plugin_opener::init;

mod hotkey;
mod text_detector;

use text_detector::TextDetector;
use tauri::{AppHandle, State};
use std::sync::Mutex;

// Commands that can be called from the frontend
#[tauri::command]
async fn start_text_detection(
    app_handle: AppHandle,
    detector_state: State<'_, Mutex<Option<TextDetector>>>,
) -> Result<String, String> {
    let mut detector = detector_state.lock().unwrap();
    
    if detector.is_none() {
        let text_detector = TextDetector::new(app_handle);
        
        // Request permissions if needed
        if let Err(e) = text_detector.request_permissions() {
            return Err(format!("Failed to request permissions: {}", e));
        }
        
        // Start the detector
        if let Err(e) = text_detector.start() {
            return Err(format!("Failed to start text detection: {}", e));
        }
        
        *detector = Some(text_detector);
        Ok("Text detection started successfully".to_string())
    } else {
        Ok("Text detection is already running".to_string())
    }
}

#[tauri::command]
async fn stop_text_detection(
    detector_state: State<'_, Mutex<Option<TextDetector>>>,
) -> Result<String, String> {
    let mut detector = detector_state.lock().unwrap();
    
    if let Some(text_detector) = detector.as_ref() {
        text_detector.stop();
        *detector = None;
        Ok("Text detection stopped".to_string())
    } else {
        Ok("Text detection was not running".to_string())
    }
}

#[tauri::command]
async fn check_permissions() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        use accessibility_sys::*;
        unsafe {
            Ok(AXIsProcessTrusted())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(None::<TextDetector>))
        .setup(|app| {
            // Register hotkeys after plugins are initialized
            hotkey::register_hotkey(&app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_text_detection,
            stop_text_detection,
            check_permissions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
