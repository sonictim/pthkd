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
