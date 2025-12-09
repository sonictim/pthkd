//! Menu bar (status bar) icon for the daemon
//!
//! Creates a menu bar icon that allows:
//! - Quitting the daemon
//! - Reloading config
//! - Other future actions

use libc::c_void;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use anyhow::Result;

/// Initialize the app as an Accessory app with a menu bar icon
///
/// This must be called early in main() before the event loop starts
pub unsafe fn setup_menu_bar_app() -> Result<()> {
    log::info!("Setting up menu bar app...");

    // Get NSApp (shared application instance)
    let ns_app_class = class!(NSApplication);
    let ns_app: *mut AnyObject = msg_send![ns_app_class, sharedApplication];

    // Set activation policy to Accessory (menu bar app)
    // NSApplicationActivationPolicyAccessory = 1
    let accessory_policy: isize = 1;
    let _: bool = msg_send![ns_app, setActivationPolicy: accessory_policy];
    log::info!("Set activation policy to Accessory (menu bar app)");

    // Create status bar item
    let status_bar_class = class!(NSStatusBar);
    let system_status_bar: *mut AnyObject = msg_send![status_bar_class, systemStatusBar];

    // NSVariableStatusItemLength = -1
    let variable_length: f64 = -1.0;
    let status_item: *mut AnyObject = msg_send![system_status_bar, statusItemWithLength: variable_length];

    // Get the status item's button
    let button: *mut AnyObject = msg_send![status_item, button];

    // Set the icon/title
    // For now, just use a simple emoji. Later we can use an actual icon image.
    let title_ns = super::ffi::create_cfstring("⌨");
    let _: () = msg_send![button, setTitle: title_ns];

    log::info!("Created menu bar icon");

    // Create the menu
    let menu_class = class!(NSMenu);
    let menu: *mut AnyObject = msg_send![menu_class, alloc];
    let menu: *mut AnyObject = msg_send![menu, init];

    // Add "Reload Config" menu item
    let reload_title = super::ffi::create_cfstring("Reload Config");
    let reload_item: *mut AnyObject = msg_send![menu,
        addItemWithTitle: reload_title
        action: std::ptr::null::<c_void>()
        keyEquivalent: super::ffi::create_cfstring("")
    ];
    // TODO: Set action/target for reload

    // Add separator
    let separator_class = class!(NSMenuItem);
    let separator: *mut AnyObject = msg_send![separator_class, separatorItem];
    let _: () = msg_send![menu, addItem: separator];

    // Add "Quit" menu item
    let quit_title = super::ffi::create_cfstring("Quit");
    let quit_item: *mut AnyObject = msg_send![menu,
        addItemWithTitle: quit_title
        action: std::ptr::null::<c_void>()
        keyEquivalent: super::ffi::create_cfstring("q")
    ];

    // Set quit action to terminate:
    // We need to set the target to NSApp and action to "terminate:"
    let terminate_selector = objc2::sel!(terminate:);
    let _: () = msg_send![quit_item, setTarget: ns_app];
    let _: () = msg_send![quit_item, setAction: terminate_selector];

    log::info!("Created menu items");

    // Attach menu to status item
    let _: () = msg_send![status_item, setMenu: menu];

    log::info!("Menu bar setup complete");

    Ok(())
}
