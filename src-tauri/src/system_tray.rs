use tauri::{AppHandle, Manager, menu::{Menu, MenuItem, PredefinedMenuItem}, tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState}};
use std::sync::Mutex;
use crate::TextDetector;

pub fn create_system_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let start_detection_item = MenuItem::with_id(app, "start_detection", "Start Detection", true, None::<&str>)?;
    let stop_detection_item = MenuItem::with_id(app, "stop_detection", "Stop Detection", true, None::<&str>)?;
    let permissions_item = MenuItem::with_id(app, "permissions", "Check Permissions", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    
    let menu = Menu::with_items(app, &[
        &show_item,
        &PredefinedMenuItem::separator(app)?,
        &start_detection_item,
        &stop_detection_item,
        &PredefinedMenuItem::separator(app)?,
        &permissions_item,
        &PredefinedMenuItem::separator(app)?,
        &quit_item,
    ])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("ACMI Desktop - Text Selection Monitor")
        .on_menu_event(move |tray, event| {
            handle_menu_event(tray.app_handle(), event);
        })
        .on_tray_icon_event(|tray, event| {
            handle_tray_click_event(tray.app_handle(), event);
        })
        .build(app)?;

    Ok(())
}

pub fn handle_tray_click_event(app: &AppHandle, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            println!("System tray received a left click");
            // Show the main window on left click
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        _ => {}
    }
}

pub fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "quit" => {
            std::process::exit(0);
        }
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "start_detection" => {
            // Start text detection
            let detector_state = app.state::<Mutex<Option<TextDetector>>>();
            let mut detector = detector_state.lock().unwrap();
            
            if detector.is_none() {
                let text_detector = TextDetector::new(app.clone());
                
                if let Ok(_) = text_detector.request_permissions() {
                    if let Ok(_) = text_detector.start() {
                        *detector = Some(text_detector);
                        println!("Text detection started from system tray");
                    }
                }
            }
        }
        "stop_detection" => {
            // Stop text detection
            let detector_state = app.state::<Mutex<Option<TextDetector>>>();
            let mut detector = detector_state.lock().unwrap();
            
            if let Some(text_detector) = detector.as_ref() {
                text_detector.stop();
                *detector = None;
                println!("Text detection stopped from system tray");
            }
        }
        "permissions" => {
            // Check permissions
            #[cfg(target_os = "macos")]
            {
                use crate::macos;
                let has_permissions = macos::check_accessibility_permissions();
                let message = if has_permissions {
                    "✅ Accessibility permissions are granted!"
                } else {
                    "❌ Accessibility permissions are required. Please grant them in System Settings."
                };
                
                println!("{}", message);
                
                if !has_permissions {
                    let _ = macos::request_accessibility_permissions();
                }
            }
        }
        _ => {}
    }
}
