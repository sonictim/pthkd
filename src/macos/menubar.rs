//! macOS menu bar integration
//!
//! Provides functionality to display a status item (icon) in the macOS menu bar.
//! Uses NSStatusBar and NSStatusItem APIs via objc2.

use anyhow::{Context, Result};
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};
use std::ptr;
use std::sync::OnceLock;

// Import session
use super::session::MacOSSession;

// Global callback for reload config
static RELOAD_CALLBACK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// C callback function that can be called from Objective-C
#[unsafe(no_mangle)]
extern "C" fn menu_reload_config(
    _this: *mut AnyObject,
    _cmd: objc2::runtime::Sel,
    _sender: *mut AnyObject,
) {
    log::info!("Reload Config menu item clicked");
    if let Some(callback) = RELOAD_CALLBACK.get() {
        callback();
    } else {
        log::error!("Reload callback not set!");
    }
}

extern "C" fn menu_restore_defaults(
    _this: *mut AnyObject,
    _cmd: objc2::runtime::Sel,
    _sender: *mut AnyObject,
) {
    log::info!("Restore Defaults menu item clicked");
    crate::config::create_default_config().ok();
}

extern "C" fn menu_show_about(
    _this: *mut AnyObject,
    _cmd: objc2::runtime::Sel,
    _sender: *mut AnyObject,
) {
    log::info!("About menu item clicked");
    unsafe {
        show_about_dialog();
    }
}

/// Represents a macOS menu bar status item
///
/// Holds a reference to the NSStatusItem which displays the icon in the menu bar.
/// The status item is automatically removed when this struct is dropped.
pub struct MenuBar {
    status_item: *mut AnyObject,
}

impl MenuBar {
    /// Get the raw status item pointer (for future menu attachment)
    #[allow(dead_code)]
    pub fn status_item(&self) -> *mut AnyObject {
        self.status_item
    }
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        unsafe {
            if !self.status_item.is_null() {
                // Remove status item from status bar
                let status_bar_class = AnyClass::get("NSStatusBar").expect("NSStatusBar class");
                let system_status_bar: *mut AnyObject =
                    msg_send![status_bar_class, systemStatusBar];

                let _: () = msg_send![system_status_bar, removeStatusItem: self.status_item];

                log::debug!("Menu bar status item removed");
            }
        }
    }
}

/// Creates a menu bar status item with an icon and menu
///
/// # Arguments
/// * `icon_data` - Optional PNG image data for custom icon. If None, uses SF Symbol fallback.
/// * `on_reload` - Callback function to call when "Reload Config" is clicked
///
/// # Icon Loading Priority
/// 1. Embedded PNG data (if provided)
/// 2. SF Symbol "bolt.fill" (macOS 11+)
/// 3. Text emoji "⚡" (fallback)
///
/// # Returns
/// A MenuBar struct containing the status item reference
///
/// # Example
/// ```ignore
/// // Use SF Symbol (default)
/// let menu_bar = create_menu_bar(None, || { println!("Reload!"); })?;
///
/// // Use custom embedded icon
/// const ICON: &[u8] = include_bytes!("../assets/icon.png");
/// let menu_bar = create_menu_bar(Some(ICON), || { println!("Reload!"); })?;
/// ```
pub unsafe fn create_menu_bar<F>(icon_data: Option<&[u8]>, on_reload: F) -> Result<MenuBar>
where
    F: Fn() + Send + Sync + 'static,
{
    // Store the callback globally
    RELOAD_CALLBACK
        .set(Box::new(on_reload))
        .map_err(|_| anyhow::anyhow!("Reload callback already set"))?;
    log::info!("Creating menu bar status item...");

    // Get the system status bar
    let status_bar_class =
        AnyClass::get("NSStatusBar").context("Failed to get NSStatusBar class")?;
    let system_status_bar: *mut AnyObject = msg_send![status_bar_class, systemStatusBar];

    if system_status_bar.is_null() {
        anyhow::bail!("Failed to get system status bar");
    }

    // Create status item with variable length
    // -1.0 = NSVariableStatusItemLength
    let status_item: *mut AnyObject = msg_send![system_status_bar, statusItemWithLength: -1.0_f64];

    if status_item.is_null() {
        anyhow::bail!("Failed to create status item");
    }

    log::debug!("Status item created successfully");

    // Get the status item's button (NSStatusBarButton)
    let button: *mut AnyObject = msg_send![status_item, button];

    if button.is_null() {
        anyhow::bail!("Failed to get status item button");
    }

    // Configure button to send action on left click
    let left_mouse_down_mask: u64 = 1 << 1; // NSEventMaskLeftMouseDown
    let _: i64 = msg_send![button, sendActionOn: left_mouse_down_mask];

    log::debug!("Button configured for left mouse clicks");

    // Load icon with fallback strategy
    let image = if let Some(png_data) = icon_data {
        log::debug!("Attempting to load custom PNG icon");
        unsafe { create_image_from_png(png_data) }
            .or_else(|| {
                log::warn!("Failed to load custom PNG, trying SF Symbol");
                unsafe { load_sf_symbol("bolt.fill") }
            })
            .or_else(|| {
                log::warn!("SF Symbol not available, using text fallback");
                unsafe { create_text_image("⚡") }
            })
    } else {
        log::debug!("No custom icon provided, using SF Symbol");
        unsafe { load_sf_symbol("bolt.fill") }.or_else(|| {
            log::warn!("SF Symbol not available, using text fallback");
            unsafe { create_text_image("⚡") }
        })
    };

    if let Some(img) = image {
        // Set the image as a template image for dark mode support
        let _: () = msg_send![img, setTemplate: true];

        // Set the image on the button
        let _: () = msg_send![button, setImage: img];

        log::info!("✅ Menu bar icon set successfully");
    } else {
        log::error!("Failed to create any icon (PNG, SF Symbol, or text)");
        anyhow::bail!("Could not create menu bar icon");
    }

    // Create delegate object for menu callbacks
    let delegate = unsafe { create_menu_delegate()? };

    // Create and attach menu
    log::info!("Creating status bar menu...");
    let menu = unsafe { create_status_menu(delegate)? };

    if menu.is_null() {
        log::error!("Menu is null!");
        anyhow::bail!("Menu creation returned null");
    }

    log::info!("Menu created, attaching to status item...");
    let _: () = msg_send![status_item, setMenu: menu];

    // Verify menu was set
    let retrieved_menu: *mut AnyObject = msg_send![status_item, menu];
    if retrieved_menu.is_null() {
        log::error!("Menu was not attached to status item!");
    } else {
        log::info!("✅ Menu bar menu created and attached successfully");
    }

    Ok(MenuBar { status_item })
}

/// Creates a delegate object to handle menu callbacks
unsafe fn create_menu_delegate() -> Result<*mut AnyObject> {
    use objc2::declare::ClassBuilder;
    use objc2::sel;

    // Try to get existing class first
    if let Some(class) = AnyClass::get("MenuBarDelegate") {
        let delegate: *mut AnyObject = msg_send![class, new];
        return Ok(delegate);
    }

    // Create new class if it doesn't exist
    let superclass = AnyClass::get("NSObject").context("Failed to get NSObject class")?;
    let mut builder = ClassBuilder::new("MenuBarDelegate", superclass)
        .context("Failed to create class builder")?;

    // Add the restoreDefaults: method
    unsafe {
        builder.add_method(
            sel!(restoreDefaults:),
            menu_restore_defaults
                as extern "C" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject),
        );

        // Add the reloadConfig: method
        builder.add_method(
            sel!(reloadConfig:),
            menu_reload_config
                as extern "C" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject),
        );

        // Add the showAbout: method
        builder.add_method(
            sel!(showAbout:),
            menu_show_about as extern "C" fn(*mut AnyObject, objc2::runtime::Sel, *mut AnyObject),
        );
    }

    let class = builder.register();
    let delegate: *mut AnyObject = msg_send![class, new];

    Ok(delegate)
}

/// Creates the menu for the status item
///
/// Menu items:
/// - "Reload Config" - Reloads the hotkey configuration
/// - "Quit" - Terminates the application
unsafe fn create_status_menu(delegate: *mut AnyObject) -> Result<*mut AnyObject> {
    log::debug!("Getting NSMenu class...");
    let menu_class = AnyClass::get("NSMenu").context("Failed to get NSMenu class")?;

    log::debug!("Allocating NSMenu...");
    let menu: *mut AnyObject = msg_send![menu_class, alloc];
    let menu: *mut AnyObject = msg_send![menu, init];

    if menu.is_null() {
        log::error!("NSMenu init returned null!");
        anyhow::bail!("Failed to create NSMenu");
    }
    log::debug!("NSMenu created successfully");

    // Create "About" menu item
    log::debug!("Creating 'About' menu item...");
    let about_item = unsafe { create_menu_item("About pthkd", "showAbout:", Some(delegate))? };
    let _: () = msg_send![menu, addItem: about_item];
    log::debug!("Added 'About' item");

    // Create separator
    let separator_class_1 =
        AnyClass::get("NSMenuItem").context("Failed to get NSMenuItem class")?;
    let separator_1: *mut AnyObject = msg_send![separator_class_1, separatorItem];
    let _: () = msg_send![menu, addItem: separator_1];

    // Create "Reload Config" menu item
    log::debug!("Creating 'Reload Config' menu item...");
    let reload_item =
        unsafe { create_menu_item("Reload Config", "reloadConfig:", Some(delegate))? };
    let _: () = msg_send![menu, addItem: reload_item];
    log::debug!("Added 'Reload Config' item");

    // Create "Restore Defaults" menu item
    log::debug!("Creating 'Restore Defaults' menu item...");
    let restore_item =
        unsafe { create_menu_item("Restore Defaults", "restoreDefaults:", Some(delegate))? };
    let _: () = msg_send![menu, addItem: restore_item];
    log::debug!("Added 'Restore Defaults' item");

    // Create separator
    log::debug!("Creating separator...");
    let separator_class = AnyClass::get("NSMenuItem").context("Failed to get NSMenuItem class")?;
    let separator: *mut AnyObject = msg_send![separator_class, separatorItem];
    let _: () = msg_send![menu, addItem: separator];
    log::debug!("Added separator");

    // Create "Quit" menu item
    log::debug!("Creating 'Quit' menu item...");
    let quit_item = unsafe { create_menu_item("Quit", "terminate:", None)? };
    let _: () = msg_send![menu, addItem: quit_item];
    log::debug!("Added 'Quit' item");

    log::debug!("Menu creation complete");
    Ok(menu)
}

/// Creates a menu item with title and action selector
///
/// Legacy wrapper that calls the session method
unsafe fn create_menu_item(
    title: &str,
    action: &str,
    target: Option<*mut AnyObject>,
) -> Result<*mut AnyObject> {
    unsafe { MacOSSession::global().create_menu_item(title, action, target) }
}

/// Creates an NSImage from PNG data
///
/// # Arguments
/// * `data` - PNG image bytes
///
/// # Returns
/// NSImage pointer or None if creation failed
unsafe fn create_image_from_png(data: &[u8]) -> Option<*mut AnyObject> {
    // Create NSData from bytes
    let ns_data_class = AnyClass::get("NSData")?;
    let ns_data: *mut AnyObject = msg_send![ns_data_class, alloc];
    let ns_data: *mut AnyObject = msg_send![
        ns_data,
        initWithBytes: data.as_ptr() as *const std::ffi::c_void
        length: data.len()
    ];

    if ns_data.is_null() {
        log::error!("Failed to create NSData from PNG bytes");
        return None;
    }

    // Create NSImage from NSData
    let ns_image_class = AnyClass::get("NSImage")?;
    let image: *mut AnyObject = msg_send![ns_image_class, alloc];
    let image: *mut AnyObject = msg_send![image, initWithData: ns_data];

    if image.is_null() {
        log::error!("Failed to create NSImage from NSData");
        return None;
    }

    log::debug!("Created NSImage from PNG data");
    Some(image)
}

/// Loads an SF Symbol by name (macOS 11+ only)
///
/// # Arguments
/// * `name` - SF Symbol name (e.g., "bolt.fill")
///
/// # Returns
/// NSImage pointer or None if not available or failed
unsafe fn load_sf_symbol(name: &str) -> Option<*mut AnyObject> {
    let ns_image_class = AnyClass::get("NSImage")?;

    // Create NSString for symbol name
    let ns_string_class = AnyClass::get("NSString")?;
    let symbol_name: *mut AnyObject = msg_send![ns_string_class, alloc];
    let symbol_name: *mut AnyObject = msg_send![
        symbol_name,
        initWithBytes: name.as_ptr() as *const std::ffi::c_void
        length: name.len()
        encoding: 4_usize  // NSUTF8StringEncoding = 4
    ];

    if symbol_name.is_null() {
        log::error!("Failed to create NSString for SF Symbol name");
        return None;
    }

    // Try to create image with SF Symbol (macOS 11+ API)
    // This will return nil on older macOS versions
    let image: *mut AnyObject = msg_send![
        ns_image_class,
        imageWithSystemSymbolName: symbol_name
        accessibilityDescription: ptr::null::<AnyObject>()
    ];

    if image.is_null() {
        log::debug!("SF Symbol '{}' not available (requires macOS 11+)", name);
        return None;
    }

    log::debug!("Loaded SF Symbol: {}", name);
    Some(image)
}

/// Creates an NSImage from text (fallback option)
///
/// # Arguments
/// * `text` - Text to render as image (e.g., "⚡")
///
/// # Returns
/// NSImage pointer or None if creation failed
///
/// Note: This is a simple fallback that creates an empty image.
/// In practice, the SF Symbol fallback should work on macOS 11+.
unsafe fn create_text_image(_text: &str) -> Option<*mut AnyObject> {
    // For simplicity, we just log that we tried
    // The SF Symbol should work on any modern macOS (11+)
    // If we get here on an older system, the status item will just be empty
    log::warn!("Text fallback requested but not fully implemented");
    log::warn!("Menu bar icon may be invisible - upgrade to macOS 11+ for SF Symbol support");

    // Return None to indicate failure
    // The status item will still be created but without an image
    None
}

// ============================================================================
// MacOSSession Extensions for Menubar
// ============================================================================

impl MacOSSession {
    /// Create an NSMenuItem with title and action
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// let item = os.create_menu_item("Quit", "terminate:", None)?;
    /// ```
    pub unsafe fn create_menu_item(
        &self,
        title: &str,
        action: &str,
        target: Option<*mut AnyObject>,
    ) -> Result<*mut AnyObject> {
        use objc2::sel;

        unsafe {
            let menu_item_class = self.get_class("NSMenuItem")?;

            // Create NSString for title using session method
            let title_string = self.create_nsstring(title)?;

            // Create selector from action string
            let selector = match action {
                "terminate:" => sel!(terminate:),
                "restoreDefaults:" => sel!(restoreDefaults:),
                "reloadConfig:" => sel!(reloadConfig:),
                "showAbout:" => sel!(showAbout:),
                _ => anyhow::bail!("Unknown action: {}", action),
            };

            // Create empty NSString for key equivalent (no keyboard shortcut)
            let ns_string_class = self.get_class("NSString")?;
            let empty_string: *mut AnyObject = msg_send![ns_string_class, string];

            // Create menu item
            let menu_item: *mut AnyObject = msg_send![menu_item_class, alloc];
            let menu_item: *mut AnyObject = msg_send![
                menu_item,
                initWithTitle: title_string
                action: selector
                keyEquivalent: empty_string
            ];

            if menu_item.is_null() {
                anyhow::bail!("Failed to create NSMenuItem");
            }

            // Set target based on action
            if let Some(target_obj) = target {
                let _: () = msg_send![menu_item, setTarget: target_obj];
                log::debug!("Set menu item target to custom delegate");
            } else if action == "terminate:" {
                let ns_app_class = self.get_class("NSApplication")?;
                let ns_app: *mut AnyObject = msg_send![ns_app_class, sharedApplication];
                let _: () = msg_send![menu_item, setTarget: ns_app];
                log::debug!("Set menu item target to NSApp for terminate:");
            }

            Ok(menu_item)
        }
    }

    /// Show an About dialog with version information
    ///
    /// # Example
    /// ```ignore
    /// let os = MacOSSession::global();
    /// os.show_about_dialog()?;
    /// ```
    pub unsafe fn show_about_dialog(&self) -> Result<()> {
        let version = env!("CARGO_PKG_VERSION");
        let message = format!(
            "ProTools Hotkey Daemon\nA fast, scriptable hotkey system for Pro Tools\n\nVersion {}",
            version
        );

        unsafe { self.show_alert("About pthkd", &message, &["OK"])? };
        Ok(())
    }
}

/// Legacy wrapper for show_about_dialog() - calls session method
unsafe fn show_about_dialog() {
    unsafe { MacOSSession::global().show_about_dialog().ok() };
}
