use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};

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
}
