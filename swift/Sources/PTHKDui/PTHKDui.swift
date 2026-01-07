import Cocoa
import ApplicationServices

// C ABI: Get menu structure of app as JSON
// Pass empty string to get frontmost app, or specify app name
@_cdecl("pthkd_get_app_menus")
public func getAppMenus(appName: UnsafePointer<CChar>?) -> UnsafePointer<CChar>? {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let json = try MenuOps.getAppMenusJSON(appName: app)
        return UnsafePointer(strdup(json))  // Rust must free this
    } catch {
        let errorJSON = "{\"error\": \"\(error.localizedDescription)\"}"
        return UnsafePointer(strdup(errorJSON))
    }
}

// C ABI: Free returned string
@_cdecl("pthkd_free_string")
public func freeString(ptr: UnsafePointer<CChar>) {
    free(UnsafeMutablePointer(mutating: ptr))
}

// C ABI: Click a menu item
// Pass empty string to get frontmost app, or specify app name
// menuPath is an array of menu titles to traverse (e.g., ["File", "Save"])
@_cdecl("pthkd_menu_click")
public func menuClick(
    appName: UnsafePointer<CChar>?,
    menuPath: UnsafePointer<UnsafePointer<CChar>?>,
    menuPathCount: Int32
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""

        // Convert menu path array to Swift [String]
        var path: [String] = []
        for i in 0..<Int(menuPathCount) {
            if let itemPtr = menuPath[i] {
                path.append(String(cString: itemPtr))
            }
        }

        try MenuOps.menuClick(appName: app, menuPath: path)
        return true
    } catch {
        NSLog("pthkd_menu_click error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Send keystroke to application
// modifiers: bit flags (shift=1, control=2, option=4, command=8)
@_cdecl("pthkd_send_keystroke")
public func sendKeystroke(
    appName: UnsafePointer<CChar>?,
    keyChar: UnsafePointer<CChar>,
    modifiers: Int32
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let key = String(cString: keyChar)

        try Keystroke.send(appName: app, keyChar: key, modifiers: Int(modifiers))
        return true
    } catch {
        NSLog("pthkd_send_keystroke error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Click a button in a window
@_cdecl("pthkd_click_button")
public func clickButton(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?,
    buttonName: UnsafePointer<CChar>
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let window = windowName != nil ? String(cString: windowName!) : ""
        let button = String(cString: buttonName)

        try WindowOps.clickButton(appName: app, windowName: window, buttonName: button)
        return true
    } catch {
        NSLog("pthkd_click_button error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Click a checkbox in a window
@_cdecl("pthkd_click_checkbox")
public func clickCheckbox(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?,
    checkboxName: UnsafePointer<CChar>
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let window = windowName != nil ? String(cString: windowName!) : ""
        let checkbox = String(cString: checkboxName)

        try WindowOps.clickCheckbox(appName: app, windowName: window, checkboxName: checkbox)
        return true
    } catch {
        NSLog("pthkd_click_checkbox error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Get list of buttons in a window (returns JSON array)
@_cdecl("pthkd_get_window_buttons")
public func getWindowButtons(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?
) -> UnsafePointer<CChar>? {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let window = windowName != nil ? String(cString: windowName!) : ""

        let buttons = try WindowOps.getWindowButtons(appName: app, windowName: window)
        let jsonData = try JSONSerialization.data(withJSONObject: buttons)
        let json = String(data: jsonData, encoding: .utf8) ?? "[]"
        return UnsafePointer(strdup(json))  // Rust must free this
    } catch {
        let errorJSON = "{\"error\": \"\(error.localizedDescription)\"}"
        return UnsafePointer(strdup(errorJSON))
    }
}

// MARK: - App Operations

// C ABI: Get frontmost app and window info (returns JSON: {app: "", window: ""})
@_cdecl("pthkd_get_frontmost_info")
public func getFrontmostInfo() -> UnsafePointer<CChar>? {
    do {
        let info = try AppOps.getFrontmostInfo()
        let dict: [String: String] = ["app": info.appName, "window": info.windowName]
        let jsonData = try JSONSerialization.data(withJSONObject: dict)
        let json = String(data: jsonData, encoding: .utf8) ?? "{}"
        return UnsafePointer(strdup(json))
    } catch {
        let errorJSON = "{\"error\": \"\(error.localizedDescription)\"}"
        return UnsafePointer(strdup(errorJSON))
    }
}

// C ABI: Get list of running apps (returns JSON array)
@_cdecl("pthkd_get_running_apps")
public func getRunningApps() -> UnsafePointer<CChar>? {
    let apps = AppOps.getRunningApps()
    do {
        let jsonData = try JSONSerialization.data(withJSONObject: apps)
        let json = String(data: jsonData, encoding: .utf8) ?? "[]"
        return UnsafePointer(strdup(json))
    } catch {
        return UnsafePointer(strdup("[]"))
    }
}

// C ABI: Focus/activate an application
@_cdecl("pthkd_focus_app")
public func focusApp(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?,
    shouldSwitch: Bool,
    shouldLaunch: Bool,
    timeout: Int32
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let window = windowName != nil ? String(cString: windowName!) : ""

        try AppOps.focusApp(
            appName: app,
            windowName: window,
            shouldSwitch: shouldSwitch,
            shouldLaunch: shouldLaunch,
            timeout: Int(timeout)
        )
        return true
    } catch {
        NSLog("pthkd_focus_app error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Launch an application
@_cdecl("pthkd_launch_app")
public func launchApp(appName: UnsafePointer<CChar>) -> Bool {
    do {
        let app = String(cString: appName)
        try AppOps.launchApp(appName: app)
        return true
    } catch {
        NSLog("pthkd_launch_app error: \(error.localizedDescription)")
        return false
    }
}

// C ABI: Check if currently focused element is a text field
@_cdecl("pthkd_is_in_text_field")
public func isInTextField() -> Bool {
    return AppOps.isInTextField()
}

// MARK: - Window Operations

// C ABI: Check if a window exists
@_cdecl("pthkd_window_exists")
public func windowExists(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?
) -> Bool {
    let app = appName != nil ? String(cString: appName!) : ""
    let window = windowName != nil ? String(cString: windowName!) : ""
    return WindowOps.windowExists(appName: app, windowName: window)
}

// C ABI: Get all window titles for an app (returns JSON array)
@_cdecl("pthkd_get_window_titles")
public func getWindowTitles(appName: UnsafePointer<CChar>?) -> UnsafePointer<CChar>? {
    let app = appName != nil ? String(cString: appName!) : ""
    let titles = WindowOps.getWindowTitles(appName: app)

    do {
        let jsonData = try JSONSerialization.data(withJSONObject: titles)
        let json = String(data: jsonData, encoding: .utf8) ?? "[]"
        return UnsafePointer(strdup(json))
    } catch {
        return UnsafePointer(strdup("[]"))
    }
}

// C ABI: Wait for window condition
// condition: 0=exists, 1=closed, 2=focused
@_cdecl("pthkd_wait_for_window")
public func waitForWindow(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?,
    condition: Int32,
    timeout: Int32
) -> Bool {
    let app = appName != nil ? String(cString: appName!) : ""
    let window = windowName != nil ? String(cString: windowName!) : ""

    let windowCondition: WindowCondition
    switch condition {
    case 0: windowCondition = .exists
    case 1: windowCondition = .closed
    case 2: windowCondition = .focused
    default: return false
    }

    return WindowOps.waitForWindow(
        appName: app,
        windowName: window,
        condition: windowCondition,
        timeout: Int(timeout)
    )
}

// C ABI: Close a window
// retryTimeout: -1 for no retry, otherwise retry for this many milliseconds
@_cdecl("pthkd_close_window")
public func closeWindow(
    appName: UnsafePointer<CChar>?,
    windowName: UnsafePointer<CChar>?,
    retryTimeout: Int32
) -> Bool {
    do {
        let app = appName != nil ? String(cString: appName!) : ""
        let window = windowName != nil ? String(cString: windowName!) : ""
        let retry = retryTimeout >= 0 ? Int(retryTimeout) : nil

        try WindowOps.closeWindow(appName: app, windowName: window, retryTimeout: retry)
        return true
    } catch {
        NSLog("pthkd_close_window error: \(error.localizedDescription)")
        return false
    }
}
