use accessibility_sys::*;
use core_foundation::string::{CFStringRef, CFString};
use core_foundation::base::{CFTypeRef, TCFType};

pub fn get_mac_selected_text() -> Option<String> {
    unsafe {
        // Check if we have accessibility permissions
        if !AXIsProcessTrusted() {
            eprintln!("Accessibility permissions not granted");
            return None;
        }

        // Create a reference to the system-wide accessibility element
        let system_wide = AXUIElementCreateSystemWide();

        // Get the currently focused UI element
        let mut focused: AXUIElementRef = std::ptr::null_mut();
        let focused_attr = CFString::new(kAXFocusedUIElementAttribute);
        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_attr.as_concrete_TypeRef(),
            &mut focused as *mut _ as *mut CFTypeRef,
        );

        if result != kAXErrorSuccess || focused.is_null() {
            println!("No focused element found");
            return None;
        }

        // Try to read the currently selected text
        let mut selected_text_ref: CFTypeRef = std::ptr::null_mut();
        let selected_attr = CFString::new(kAXSelectedTextAttribute);
        let result = AXUIElementCopyAttributeValue(
            focused,
            selected_attr.as_concrete_TypeRef(),
            &mut selected_text_ref,
        );

        if result == kAXErrorSuccess && !selected_text_ref.is_null() {
            // Convert CFTypeRef -> CFString -> Rust String
            let cf_string: CFString = TCFType::wrap_under_create_rule(selected_text_ref as CFStringRef);
            let text = cf_string.to_string();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }

        // If no selected text, try to get the value or title
        let attributes = [kAXValueAttribute, kAXTitleAttribute];
        for attr_name in &attributes {
            let mut text_ref: CFTypeRef = std::ptr::null_mut();
            let attr = CFString::new(attr_name);
            let result = AXUIElementCopyAttributeValue(focused, attr.as_concrete_TypeRef(), &mut text_ref);

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
}
