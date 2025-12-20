use anyhow::{Result, anyhow};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

pub fn password_set(account: &str, password: &str) -> Result<()> {
    let service = "com.feralfrequencies.pthkd";

    log::info!("password_set: service='{}', account='{}'", service, account);

    // Delete existing password if it exists (to avoid duplicates)
    delete_generic_password(service, account).ok();

    log::info!("password_set: Attempting to set password using Security framework");

    match set_generic_password(service, account, password.as_bytes()) {
        Ok(_) => {
            log::info!("password_set: Password set successfully in keychain");
            Ok(())
        }
        Err(e) => {
            log::error!("password_set: Failed to set password in keychain: {}", e);
            Err(anyhow!("Failed to set password: {}", e))
        }
    }
}

pub fn password_get(account: &str) -> Result<String> {
    let service = "com.feralfrequencies.pthkd";

    log::info!("password_get: service='{}', account='{}'", service, account);

    match get_generic_password(service, account) {
        Ok(password_bytes) => {
            let password = String::from_utf8(password_bytes.to_vec())
                .map_err(|e| anyhow!("Failed to decode password: {}", e))?;
            log::info!("password_get: Password retrieved successfully");
            Ok(password)
        }
        Err(e) => {
            log::error!("password_get: Failed to get password: {}", e);
            Err(anyhow!("Failed to get password: {}", e))
        }
    }
}

pub fn password_prompt(account: &str) -> Result<()> {
    use objc2::{msg_send, runtime::AnyClass, runtime::AnyObject};
    use std::ffi::c_void;

    unsafe {
        // Get NSAlert class
        let alert_class = AnyClass::get("NSAlert")
            .ok_or_else(|| anyhow::anyhow!("Failed to get NSAlert class"))?;

        // Create alert
        let alert: *mut AnyObject = msg_send![alert_class, alloc];
        let alert: *mut AnyObject = msg_send![alert, init];

        // Set message text
        let message = "Please enter password to store in keychain:";
        let message_string = create_nsstring(message);
        let _: () = msg_send![alert, setMessageText: message_string];

        // Create a secure text field for password input
        let text_field_class = AnyClass::get("NSSecureTextField")
            .ok_or_else(|| anyhow::anyhow!("Failed to get NSSecureTextField class"))?;

        let text_field: *mut AnyObject = msg_send![text_field_class, alloc];
        let text_field: *mut AnyObject = msg_send![text_field, init];

        // Set a reasonable width for the text field (NSTextField needs explicit sizing)
        // We'll use sizeToFit and then adjust the width
        let _: () = msg_send![text_field, sizeToFit];

        // Get current frame to adjust width
        use objc2::encode::{Encode, Encoding};

        #[repr(C)]
        #[derive(Copy, Clone, Debug)]
        struct NSRect {
            x: f64,
            y: f64,
            width: f64,
            height: f64,
        }

        unsafe impl Encode for NSRect {
            const ENCODING: Encoding = Encoding::Struct(
                "NSRect",
                &[
                    Encoding::Double,
                    Encoding::Double,
                    Encoding::Double,
                    Encoding::Double,
                ],
            );
        }

        let mut frame: NSRect = msg_send![text_field, frame];
        frame.width = 300.0; // Set width to 300 pixels
        let _: () = msg_send![text_field, setFrame: frame];

        // Set the text field as the accessory view
        let _: () = msg_send![alert, setAccessoryView: text_field];

        // Add buttons
        let ok_string = create_nsstring("OK");
        let cancel_string = create_nsstring("Cancel");
        let _: () = msg_send![alert, addButtonWithTitle: ok_string];
        let _: () = msg_send![alert, addButtonWithTitle: cancel_string];

        // Show the alert and get response
        let response: isize = msg_send![alert, runModal];

        // NSAlertFirstButtonReturn = 1000, NSAlertSecondButtonReturn = 1001
        if response == 1000 {
            // User clicked OK - get the password
            let password_nsstring: *mut AnyObject = msg_send![text_field, stringValue];

            log::info!(
                "Retrieved password NSString pointer: {:?}",
                password_nsstring
            );

            match nsstring_to_string(password_nsstring) {
                Some(password) if !password.is_empty() => {
                    log::info!("Password retrieved, length: {}", password.len());
                    match password_set(account, &password) {
                        Ok(_) => {
                            println!("Password stored successfully!");
                            log::info!("Password stored successfully for account: {}", account);
                        }
                        Err(e) => {
                            log::error!("Failed to store password: {}", e);
                            return Err(e);
                        }
                    }
                }
                Some(password) => {
                    log::warn!("Password was empty");
                    println!("Password was empty, not stored");
                }
                None => {
                    log::error!("Failed to convert NSString to Rust string");
                    return Err(anyhow!("Failed to retrieve password from dialog"));
                }
            }
        } else {
            log::info!("User cancelled password dialog");
        }

        Ok(())
    }
}

// Helper to create NSString from &str
unsafe fn create_nsstring(s: &str) -> *mut objc2::runtime::AnyObject {
    use objc2::{msg_send, runtime::AnyClass, runtime::AnyObject};
    use std::ffi::c_void;

    let ns_string_class = AnyClass::get("NSString").expect("NSString class");
    let string: *mut AnyObject = msg_send![ns_string_class, alloc];
    let string: *mut AnyObject = msg_send![
        string,
        initWithBytes: s.as_ptr() as *const c_void
        length: s.len()
        encoding: 4_usize  // NSUTF8StringEncoding
    ];
    string
}

// Helper to convert NSString to Rust String
unsafe fn nsstring_to_string(ns_string: *mut objc2::runtime::AnyObject) -> Option<String> {
    use objc2::{msg_send, runtime::AnyObject};

    if ns_string.is_null() {
        log::error!("nsstring_to_string: NSString pointer is null");
        return None;
    }

    let utf8: *const u8 = msg_send![ns_string, UTF8String];
    if utf8.is_null() {
        log::error!("nsstring_to_string: UTF8String method returned null");
        return None;
    }

    let c_str = std::ffi::CStr::from_ptr(utf8 as *const i8);
    match c_str.to_str() {
        Ok(s) => {
            log::info!(
                "nsstring_to_string: Successfully converted string, length: {}",
                s.len()
            );
            Some(s.to_string())
        }
        Err(e) => {
            log::error!("nsstring_to_string: Failed to convert CStr to str: {}", e);
            None
        }
    }
}
