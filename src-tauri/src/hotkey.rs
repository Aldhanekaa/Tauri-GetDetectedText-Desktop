use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri::Emitter;

#[cfg(target_os = "macos")]
mod mac_a11y;

pub fn register_hotkey(app: &AppHandle) {
    let shortcut_str = if cfg!(target_os = "macos") { "Command+Shift+L" } else { "Ctrl+Shift+L" };
    
    // Try to get the main window
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            eprintln!("Main window not found");
            return;
        }
    };

    let global_shortcut = app.global_shortcut();
    
    match shortcut_str.parse::<Shortcut>() {
        Ok(parsed_shortcut) => {
            match global_shortcut.register(parsed_shortcut) {
                Ok(_) => {
                    println!("Hotkey {} registered successfully", shortcut_str);
                    
                    let _ = global_shortcut.on_shortcut(parsed_shortcut, move |_app, _hotkey, _event| {
                        println!("Hotkey triggered!");
                        
                        #[cfg(target_os = "macos")]
                        if let Some(text) = mac_a11y::get_mac_selected_text() {
                            println!("Selected text: {}", text);
                            let _ = window.emit("hotkey-selection-detected", text);
                        }
                        
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = window.emit("hotkey-triggered", "Hotkey pressed");
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to register hotkey {}: {}", shortcut_str, e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse hotkey {}: {}", shortcut_str, e);
        }
    }
}