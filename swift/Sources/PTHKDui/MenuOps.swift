import Cocoa
import ApplicationServices

enum MenuError: Error {
    case noFrontmostApp
    case appNotFound(String)
    case menuBarNotFound
    case menuItemNotFound(String)
    case emptyMenuPath
    case menuClickFailed
}

class MenuOps {
    static func getAppMenusJSON(appName: String) throws -> String {
        let app: NSRunningApplication
        let displayName: String

        if appName.isEmpty {
            // Get frontmost app
            guard let frontmost = NSWorkspace.shared.frontmostApplication else {
                throw MenuError.noFrontmostApp
            }
            app = frontmost
            displayName = app.localizedName ?? "Unknown"
        } else {
            // Find app by name
            let runningApps = NSWorkspace.shared.runningApplications
            guard let foundApp = runningApps.first(where: { $0.localizedName == appName }) else {
                throw MenuError.appNotFound(appName)
            }
            app = foundApp
            displayName = appName
        }

        let appElement = AXUIElementCreateApplication(app.processIdentifier)

        // Get menu bar
        var menuBarRef: AnyObject?
        let result = AXUIElementCopyAttributeValue(
            appElement,
            kAXMenuBarAttribute as CFString,
            &menuBarRef
        )

        guard result == .success, menuBarRef != nil else {
            throw MenuError.menuBarNotFound
        }

        let menuBar = menuBarRef as! AXUIElement

        // Get children (top-level menus)
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(menuBar, kAXChildrenAttribute as CFString, &childrenRef)

        guard let children = childrenRef else {
            return "{\"app\": \"\(appName)\", \"menus\": []}"
        }

        let childrenArray = children as! CFArray
        let count = CFArrayGetCount(childrenArray)
        var elementArray: [AXUIElement] = []
        for i in 0..<count {
            let element = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
            elementArray.append(element)
        }

        // Build menu structure
        var menus: [[String: Any]] = []
        for child in elementArray {
            if let menuDict = getMenuItemInfo(child) {
                menus.append(menuDict)
            }
        }

        let resultDict: [String: Any] = [
            "app": displayName,
            "menus": menus
        ]

        let jsonData = try JSONSerialization.data(withJSONObject: resultDict, options: .prettyPrinted)
        return String(data: jsonData, encoding: .utf8) ?? "{}"
    }

    private static func getMenuItemInfo(_ element: AXUIElement) -> [String: Any]? {
        var titleRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef)

        let title = (titleRef as? String) ?? "(no title)"

        var info: [String: Any] = ["title": title]

        // Try to get children (submenu items)
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef)

        if let childrenCF = childrenRef {
            let childrenArray = childrenCF as! CFArray
            let count = CFArrayGetCount(childrenArray)
            if count > 0 {
                var childMenus: [[String: Any]] = []
                let limit = min(count, 5)  // Limit to first 5 for POC
                for i in 0..<limit {
                    let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
                    if let childInfo = getMenuItemInfo(child) {
                        childMenus.append(childInfo)
                    }
                }
                if !childMenus.isEmpty {
                    info["children"] = childMenus
                }
            }
        }

        return info
    }

    // Click a menu item by traversing the menu path
    static func menuClick(appName: String, menuPath: [String]) throws {
        guard !menuPath.isEmpty else {
            throw MenuError.emptyMenuPath
        }

        let app: NSRunningApplication

        if appName.isEmpty {
            // Get frontmost app
            guard let frontmost = NSWorkspace.shared.frontmostApplication else {
                throw MenuError.noFrontmostApp
            }
            app = frontmost
        } else {
            // Find app by name
            let runningApps = NSWorkspace.shared.runningApplications
            guard let foundApp = runningApps.first(where: { $0.localizedName == appName }) else {
                throw MenuError.appNotFound(appName)
            }
            app = foundApp
        }

        let appElement = AXUIElementCreateApplication(app.processIdentifier)

        // Get menu bar
        var menuBarRef: AnyObject?
        let result = AXUIElementCopyAttributeValue(
            appElement,
            kAXMenuBarAttribute as CFString,
            &menuBarRef
        )

        guard result == .success, menuBarRef != nil else {
            throw MenuError.menuBarNotFound
        }

        let menuBar = menuBarRef as! AXUIElement

        // Get top-level menu items
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(menuBar, kAXChildrenAttribute as CFString, &childrenRef)

        guard let children = childrenRef else {
            throw MenuError.menuItemNotFound(menuPath[0])
        }

        let childrenArray = children as! CFArray
        let count = CFArrayGetCount(childrenArray)
        var elementArray: [AXUIElement] = []
        for i in 0..<count {
            let element = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
            elementArray.append(element)
        }

        // Start traversing from the top level
        var currentElement: AXUIElement?

        for (index, menuTitle) in menuPath.enumerated() {
            let searchIn = currentElement == nil ? elementArray : getChildren(of: currentElement!)

            // Find menu item by title
            var found: AXUIElement?
            for element in searchIn {
                var titleRef: AnyObject?
                AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef)
                let title = (titleRef as? String) ?? ""

                if title == menuTitle {
                    found = element
                    break
                }
            }

            guard let menuItem = found else {
                throw MenuError.menuItemNotFound(menuTitle)
            }

            // If this is the last item in the path, click it
            if index == menuPath.count - 1 {
                let pressResult = AXUIElementPerformAction(menuItem, kAXPressAction as CFString)
                guard pressResult == .success else {
                    throw MenuError.menuClickFailed
                }
                return
            }

            // Otherwise, continue traversing
            currentElement = menuItem
        }
    }

    // Helper to get children of an element
    private static func getChildren(of element: AXUIElement) -> [AXUIElement] {
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef)

        guard let childrenCF = childrenRef else {
            return []
        }

        let childrenArray = childrenCF as! CFArray
        let count = CFArrayGetCount(childrenArray)
        var result: [AXUIElement] = []

        for i in 0..<count {
            let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
            result.append(child)
        }

        return result
    }
}
