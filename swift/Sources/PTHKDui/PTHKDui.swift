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
