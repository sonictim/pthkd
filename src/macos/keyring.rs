use anyhow::{Result, bail};
use keyring::Entry;

pub fn password_set(account: &str, password: &str) -> Result<()> {
    let service = "com.feralfrequencies.pthkd"; // use a reverse-DNS identifier

    let entry = Entry::new(service, account)?;
    entry.set_password(password)?;
    Ok(())
}

pub fn password_get(account: &str) -> Result<String> {
    let service = "com.feralfrequencies.pthkd";

    let entry = Entry::new(service, account)?;
    Ok(entry.get_password()?)
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
            if let Some(password) = nsstring_to_string(password_nsstring)
                && !password.is_empty()
            {
                password_set(account, &password)?;
                println!("Password stored successfully!");
            }
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
        return None;
    }

    let utf8: *const u8 = msg_send![ns_string, UTF8String];
    if utf8.is_null() {
        return None;
    }

    let c_str = std::ffi::CStr::from_ptr(utf8 as *const i8);
    c_str.to_str().ok().map(|s| s.to_string())
}
