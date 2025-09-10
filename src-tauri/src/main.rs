// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri::{AppHandle, State, Manager, Emitter};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};

mod system_tray;

// Data structures for text detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionEvent {
    pub text: String,
    pub app_name: String,
    pub timestamp: u64,
    pub selection_type: SelectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionType {
    Selected,
    Hovered,
    Focused,
}

// Text detector structure
pub struct TextDetector {
    app_handle: AppHandle,
    is_running: Arc<Mutex<bool>>,
    last_selection: Arc<Mutex<Option<String>>>,
}

impl TextDetector {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            is_running: Arc::new(Mutex::new(false)),
            last_selection: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        let app_handle = self.app_handle.clone();
        let is_running_clone = Arc::clone(&self.is_running);
        let last_selection_clone = Arc::clone(&self.last_selection);

        // Check for accessibility permissions first
        #[cfg(target_os = "macos")]
        if !self.check_accessibility_permissions() {
            return Err("Accessibility permissions not granted".into());
        }

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                let is_running = {
                    let guard = is_running_clone.lock().unwrap();
                    *guard
                };
                
                if !is_running {
                    break;
                }

                if let Some(selection) = Self::get_current_selection().await {
                    let mut last = last_selection_clone.lock().unwrap();
                    
                    // Only emit if the selection has changed
                    if last.as_ref() != Some(&selection.text) {
                        *last = Some(selection.text.clone());
                        let _ = app_handle.emit("text-selection-changed", &selection);
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
    }

    async fn get_current_selection() -> Option<SelectionEvent> {
        #[cfg(target_os = "macos")]
        return macos::get_selection().await;
        
        #[cfg(not(target_os = "macos"))]
        None
    }

    #[cfg(target_os = "macos")]
    fn check_accessibility_permissions(&self) -> bool {
        macos::check_accessibility_permissions()
    }

    #[cfg(not(target_os = "macos"))]
    fn check_accessibility_permissions(&self) -> bool {
        true // Assume permissions are OK on other platforms
    }

    pub fn request_permissions(&self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        return macos::request_accessibility_permissions();
        
        #[cfg(not(target_os = "macos"))]
        Ok(())
    }
}

// Platform-specific implementations
#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;
    use accessibility_sys::*;
    use core_foundation::string::{CFStringRef, CFString};
    use core_foundation::base::{CFTypeRef, TCFType};
    
    pub async fn get_selection() -> Option<SelectionEvent> {
        unsafe {
            let system_wide = AXUIElementCreateSystemWide();
            let mut focused: AXUIElementRef = std::ptr::null_mut();
            
            // Create CFString for the attribute
            let focused_attr = CFString::new(kAXFocusedUIElementAttribute);
            let result = AXUIElementCopyAttributeValue(
                system_wide,
                focused_attr.as_concrete_TypeRef(),
                &mut focused as *mut _ as *mut CFTypeRef,
            );

            if result != kAXErrorSuccess || focused.is_null() {
                return None;
            }

            // Try to get selected text first
            if let Some(text) = get_selected_text(focused) {
                return Some(SelectionEvent {
                    text,
                    app_name: "Unknown".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    selection_type: SelectionType::Selected,
                });
            }

            // If no selected text, try to get focused text or value
            if let Some(text) = get_focused_text(focused) {
                return Some(SelectionEvent {
                    text,
                    app_name: "Unknown".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    selection_type: SelectionType::Focused,
                });
            }

            None
        }
    }

    unsafe fn get_selected_text(element: AXUIElementRef) -> Option<String> {
        let mut selected_text_ref: CFTypeRef = std::ptr::null_mut();
        let selected_attr = CFString::new(kAXSelectedTextAttribute);
        let result = AXUIElementCopyAttributeValue(
            element,
            selected_attr.as_concrete_TypeRef(),
            &mut selected_text_ref,
        );

        if result == kAXErrorSuccess && !selected_text_ref.is_null() {
            let cf_string: CFString = TCFType::wrap_under_create_rule(selected_text_ref as CFStringRef);
            let text = cf_string.to_string();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
        None
    }

    unsafe fn get_focused_text(element: AXUIElementRef) -> Option<String> {
        // Try different attributes that might contain text
        let attributes = [
            kAXValueAttribute,
            kAXTitleAttribute,
            kAXDescriptionAttribute,
            kAXHelpAttribute,
        ];

        for attr_name in &attributes {
            let mut text_ref: CFTypeRef = std::ptr::null_mut();
            let attr = CFString::new(attr_name);
            let result = AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut text_ref);

            if result == kAXErrorSuccess && !text_ref.is_null() {
                let cf_string: CFString = TCFType::wrap_under_create_rule(text_ref as CFStringRef);
                let text = cf_string.to_string();
                if !text.trim().is_empty() && text.len() > 2 {
                    return Some(text);
                }
            }
        }
        None
    }

    pub fn check_accessibility_permissions() -> bool {
        unsafe {
            AXIsProcessTrusted()
        }
    }

    pub fn request_accessibility_permissions() -> Result<(), String> {
        unsafe {
            // Use a null pointer instead of creating a dictionary to avoid type conflicts
            AXIsProcessTrustedWithOptions(std::ptr::null());
        }
        Ok(())
    }

    // Hotkey-specific function to get currently selected text
    pub fn get_mac_selected_text() -> Option<String> {
        unsafe {
            let system_wide = AXUIElementCreateSystemWide();
            let mut focused: AXUIElementRef = std::ptr::null_mut();
            
            let focused_attr = CFString::new(kAXFocusedUIElementAttribute);
            let result = AXUIElementCopyAttributeValue(
                system_wide,
                focused_attr.as_concrete_TypeRef(),
                &mut focused as *mut _ as *mut CFTypeRef,
            );

            if result != kAXErrorSuccess || focused.is_null() {
                return None;
            }

            get_selected_text(focused)
        }
    }
}

// Hotkey registration function
fn register_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut_str = if cfg!(target_os = "macos") { "Command+Shift+L" } else { "Ctrl+Shift+L" };
    
    let global_shortcut = app.global_shortcut();
    let app_clone = app.clone();
    
    match shortcut_str.parse::<Shortcut>() {
        Ok(parsed_shortcut) => {
            global_shortcut.register(parsed_shortcut)?;
            println!("Hotkey {} registered successfully", shortcut_str);
            
            let _ = global_shortcut.on_shortcut(parsed_shortcut, move |_app, _hotkey, _event| {
                println!("Hotkey triggered!");
                
                #[cfg(target_os = "macos")]
                if let Some(text) = macos::get_mac_selected_text() {
                    println!("Selected text via hotkey: {}", text);
                    let selection_event = SelectionEvent {
                        text: text.clone(),
                        app_name: "Hotkey".to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        selection_type: SelectionType::Selected,
                    };
                    let _ = app_clone.emit("hotkey-selection-detected", &selection_event);
                }
                
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = app_clone.emit("hotkey-triggered", "Hotkey pressed");
                }
            });
            
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to parse hotkey {}: {}", shortcut_str, e).into())
        }
    }
}

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
        Ok(macos::check_accessibility_permissions())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

#[tauri::command]
async fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_current_selection() -> Result<Option<SelectionEvent>, String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(text) = macos::get_mac_selected_text() {
            Ok(Some(SelectionEvent {
                text,
                app_name: "Manual".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                selection_type: SelectionType::Selected,
            }))
        } else {
            Ok(None)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Mutex::new(None::<TextDetector>))
        .invoke_handler(tauri::generate_handler![
            start_text_detection,
            stop_text_detection,
            check_permissions,
            show_main_window,
            hide_main_window,
            get_current_selection
        ])
        .setup(|app| {
            // Register global hotkey with proper error handling
            if let Err(e) = register_hotkey(&app.handle()) {
                eprintln!("Failed to register hotkey: {}", e);
            }
            
            // Create system tray
            system_tray::create_system_tray(&app.handle())?;
            
            // Hide the main window on startup to start as menu bar app
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
