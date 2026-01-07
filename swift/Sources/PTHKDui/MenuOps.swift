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

        let pid = app.processIdentifier
        let appElement = AXUIElementCreateApplication(pid)

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
            return "{\"app\": \"\(displayName)\", \"menus\": []}"
        }

        let childrenArray = children as! CFArray
        let count = CFArrayGetCount(childrenArray)
        var elementArray: [AXUIElement] = []
        for i in 0..<count {
            let element = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
            elementArray.append(element)
        }

        // Build menu structure using MenuItem
        var menus: [MenuItem] = []
        for element in elementArray {
            let menuItem = MenuItem(element: element, pid: pid, path: [])
            menus.append(menuItem)
        }

        // Encode to JSON
        let encoder = JSONEncoder()
        encoder.outputFormatting = .prettyPrinted
        let menusData = try encoder.encode(menus)

        let resultDict: [String: Any] = [
            "app": displayName,
            "menus": try JSONSerialization.jsonObject(with: menusData) as! [[String: Any]]
        ]

        let jsonData = try JSONSerialization.data(withJSONObject: resultDict, options: .prettyPrinted)
        return String(data: jsonData, encoding: .utf8) ?? "{}"
    }

    // Click a menu item using keyboard shortcut if available, or direct AXUIElement action
    static func menuClick(appName: String, menuPath: [String]) throws {
        guard !menuPath.isEmpty else {
            throw MenuError.emptyMenuPath
        }

        let app: NSRunningApplication

        if appName.isEmpty {
            guard let frontmost = NSWorkspace.shared.frontmostApplication else {
                throw MenuError.noFrontmostApp
            }
            app = frontmost
        } else {
            let runningApps = NSWorkspace.shared.runningApplications
            guard let foundApp = runningApps.first(where: { $0.localizedName == appName }) else {
                throw MenuError.appNotFound(appName)
            }
            app = foundApp
        }

        let pid = app.processIdentifier
        let appElement = AXUIElementCreateApplication(pid)

        // Get menu bar
        var menuBarRef: AnyObject?
        guard AXUIElementCopyAttributeValue(appElement, kAXMenuBarAttribute as CFString, &menuBarRef) == .success,
              menuBarRef != nil else {
            throw MenuError.menuBarNotFound
        }

        let menuBar = menuBarRef as! AXUIElement

        // Traverse to find the menu item
        var currentElement: AXUIElement = menuBar
        for (index, menuTitle) in menuPath.enumerated() {
            guard let found = findChildByTitle(currentElement, title: menuTitle) else {
                throw MenuError.menuItemNotFound(menuTitle)
            }

            // If this is the last item in the path, execute it
            if index == menuPath.count - 1 {
                try executeMenuItem(found, appName: app.localizedName ?? appName)
                return
            }

            currentElement = found
        }
    }

    /// Execute a menu item using AXPress (direct execution, no UI interaction)
    private static func executeMenuItem(_ element: AXUIElement, appName: String) throws {
        // Try AXPress (most common and fastest)
        var pressResult = AXUIElementPerformAction(element, kAXPressAction as CFString)
        if pressResult == .success {
            return
        }

        // Try AXPick as fallback (some apps use this)
        pressResult = AXUIElementPerformAction(element, kAXPickAction as CFString)
        if pressResult == .success {
            return
        }

        // If both failed, log what actions are available for debugging
        var actionsRef: CFArray?
        if AXUIElementCopyActionNames(element, &actionsRef) == .success, let actionsCF = actionsRef {
            let count = CFArrayGetCount(actionsCF)
            NSLog("Menu item execute failed. Available actions (\(count)):")
            for i in 0..<count {
                let action = unsafeBitCast(CFArrayGetValueAtIndex(actionsCF, i), to: CFString.self) as String
                NSLog("  \(action)")
            }
        }

        throw MenuError.menuClickFailed
    }

    /// Find a child element by title, skipping through "(no title)" container layers
    private static func findChildByTitle(_ element: AXUIElement, title: String) -> AXUIElement? {
        var childrenRef: AnyObject?
        guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
              childrenRef != nil else {
            return nil
        }

        let childrenCF = childrenRef as! CFArray
        let count = CFArrayGetCount(childrenCF)

        // Special case: if there's only one child with no title, it's likely the AXMenu container
        // Skip through it to get the actual menu items
        if count == 1 {
            let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, 0), to: AXUIElement.self)
            var titleRef: AnyObject?
            AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef)
            let childTitle = (titleRef as? String) ?? ""

            if childTitle.isEmpty || childTitle == "(no title)" {
                NSLog("Skipping container layer, searching in its children")
                // Search in the container's children instead
                var containerChildrenRef: AnyObject?
                if AXUIElementCopyAttributeValue(child, kAXChildrenAttribute as CFString, &containerChildrenRef) == .success,
                   containerChildrenRef != nil {
                    let containerChildrenCF = containerChildrenRef as! CFArray
                    let containerCount = CFArrayGetCount(containerChildrenCF)

                    for i in 0..<containerCount {
                        let grandchild = unsafeBitCast(CFArrayGetValueAtIndex(containerChildrenCF, i), to: AXUIElement.self)
                        var grandchildTitleRef: AnyObject?
                        if AXUIElementCopyAttributeValue(grandchild, kAXTitleAttribute as CFString, &grandchildTitleRef) == .success,
                           let grandchildTitle = grandchildTitleRef as? String,
                           grandchildTitle == title {
                            return grandchild
                        }
                    }
                }
                return nil
            }
        }

        // Normal case: search in direct children
        for i in 0..<count {
            let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)

            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef) == .success,
               let childTitle = titleRef as? String,
               childTitle == title {
                return child
            }
        }

        return nil
    }
}
