import Cocoa
import ApplicationServices

/// Attempts to directly execute menu items without UI clicking
class DirectMenuExecution {

    /// Try to execute a menu item by sending it directly to the target app
    /// This uses lower-level APIs to avoid UI automation
    static func executeMenuDirect(appName: String, menuPath: [String]) throws {
        guard !menuPath.isEmpty else {
            throw NSError(domain: "DirectMenuExecution", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Empty menu path"])
        }

        let runningApps = NSWorkspace.shared.runningApplications
        guard let app = runningApps.first(where: { $0.localizedName == appName }) else {
            throw NSError(domain: "DirectMenuExecution", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "App not found: \(appName)"])
        }

        let pid = app.processIdentifier

        // Strategy: Use Carbon Event Manager to send menu command events
        // This is lower-level than Accessibility and doesn't require UI clicking

        // First, we need to find the menu command ID
        // Unfortunately, without the app's cooperation, we can't get command IDs directly
        // So we'll have to use a hybrid approach:

        // 1. Use AX to navigate to the menu item
        // 2. Get the menu item's parent menu
        // 3. Get the item's index in that menu
        // 4. Send a Carbon event to execute that menu index

        let appElement = AXUIElementCreateApplication(pid)

        // Navigate to the menu item
        guard let menuItemElement = findMenuItem(appElement: appElement, path: menuPath) else {
            throw NSError(domain: "DirectMenuExecution", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Menu item not found"])
        }

        // Try to get the menu item's index
        var indexRef: AnyObject?
        if AXUIElementCopyAttributeValue(menuItemElement, kAXIndexAttribute as CFString, &indexRef) == .success,
           let index = indexRef as? Int {
            NSLog("Menu item index: \(index)")

            // Try to perform the action directly
            let result = AXUIElementPerformAction(menuItemElement, kAXPressAction as CFString)
            if result == .success {
                return
            }

            // If that failed, try kAXPickAction
            let pickResult = AXUIElementPerformAction(menuItemElement, kAXPickAction as CFString)
            if pickResult == .success {
                return
            }

            NSLog("AXPress result: \(result.rawValue), AXPick result: \(pickResult.rawValue)")
        }

        // If all else fails, throw error
        throw NSError(domain: "DirectMenuExecution", code: -1,
                     userInfo: [NSLocalizedDescriptionKey: "Failed to execute menu item"])
    }

    private static func findMenuItem(appElement: AXUIElement, path: [String]) -> AXUIElement? {
        // Get menu bar
        var menuBarRef: AnyObject?
        guard AXUIElementCopyAttributeValue(appElement, kAXMenuBarAttribute as CFString, &menuBarRef) == .success,
              menuBarRef != nil else {
            return nil
        }

        let menuBar = menuBarRef as! AXUIElement
        var currentElement: AXUIElement = menuBar

        // Navigate through path
        for (index, title) in path.enumerated() {
            var childrenRef: AnyObject?
            guard AXUIElementCopyAttributeValue(currentElement, kAXChildrenAttribute as CFString, &childrenRef) == .success,
                  childrenRef != nil else {
                return nil
            }

            let childrenCF = childrenRef as! CFArray
            let count = CFArrayGetCount(childrenCF)
            var found: AXUIElement?

            for i in 0..<count {
                let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)

                var titleRef: AnyObject?
                if AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef) == .success,
                   let childTitle = titleRef as? String,
                   childTitle == title {
                    found = child
                    break
                }
            }

            guard let menuItem = found else {
                return nil
            }

            // If this is the last item, return it
            if index == path.count - 1 {
                return menuItem
            }

            currentElement = menuItem
        }

        return nil
    }
}
