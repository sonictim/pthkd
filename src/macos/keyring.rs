use anyhow::{Result, anyhow};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

// Import session types
use super::session::{MacOSSession, NSRect};

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

// ============================================================================
// MacOSSession Extensions for Keyring
// ============================================================================

impl MacOSSession {
    /// Prompt for password with secure text field
    ///
    /// Shows an alert dialog with a secure text field for password entry.
    /// If the user clicks OK and enters a non-empty password, it's stored
    /// in the keychain for the given account.
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// os.password_prompt("my_account")?;
    /// ```
    pub unsafe fn password_prompt(&self, account: &str) -> Result<()> {
        use objc2::{msg_send, runtime::AnyObject};

        // Create alert using session method
        let alert_class = self.get_class("NSAlert")?;
        let alert = self.alloc_init(alert_class)?;

        // Set message text using session method
        let message = "Please enter password to store in keychain:";
        let message_string = self.create_nsstring(message)?;
        let _: () = msg_send![alert, setMessageText: message_string];

        // Create a secure text field for password input
        let text_field_class = self.get_class("NSSecureTextField")?;
        let text_field = self.alloc_init(text_field_class)?;

        // Set a reasonable width for the text field
        let _: () = msg_send![text_field, sizeToFit];

        // Get current frame and adjust width
        let mut frame: NSRect = msg_send![text_field, frame];
        frame.size.width = 300.0; // Set width to 300 pixels
        let _: () = msg_send![text_field, setFrame: frame];

        // Set the text field as the accessory view
        let _: () = msg_send![alert, setAccessoryView: text_field];

        // Add buttons using session method
        let ok_string = self.create_nsstring("OK")?;
        let cancel_string = self.create_nsstring("Cancel")?;
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

// ============================================================================
// Local Helpers
// ============================================================================

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

    let c_str = unsafe { std::ffi::CStr::from_ptr(utf8 as *const i8) };
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
