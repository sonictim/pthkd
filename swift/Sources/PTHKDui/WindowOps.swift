import Cocoa
import ApplicationServices

enum WindowError: Error {
    case noFrontmostApp
    case appNotFound(String)
    case windowNotFound(String)
    case buttonNotFound(String)
    case checkboxNotFound(String)
    case clickFailed
    case closeFailed
    case timeout
}

enum WindowCondition {
    case exists
    case closed
    case focused
}

class WindowOps {

    /// Click a button in a window
    /// - Parameters:
    ///   - appName: Name of the app (empty string for frontmost app)
    ///   - windowName: Name of the window (empty string for frontmost window)
    ///   - buttonName: Name of the button to click
    static func clickButton(appName: String, windowName: String, buttonName: String) throws {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        let button = try findButton(in: window, buttonName: buttonName)

        // Perform AXPress action
        let result = AXUIElementPerformAction(button, kAXPressAction as CFString)
        guard result == .success else {
            // Try AXPick as fallback
            let pickResult = AXUIElementPerformAction(button, kAXPickAction as CFString)
            guard pickResult == .success else {
                throw WindowError.clickFailed
            }
            return
        }
    }

    /// Click a checkbox in a window
    /// - Parameters:
    ///   - appName: Name of the app (empty string for frontmost app)
    ///   - windowName: Name of the window (empty string for frontmost window)
    ///   - checkboxName: Name of the checkbox to click
    static func clickCheckbox(appName: String, windowName: String, checkboxName: String) throws {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        let checkbox = try findCheckbox(in: window, checkboxName: checkboxName)

        // Perform AXPress action
        let result = AXUIElementPerformAction(checkbox, kAXPressAction as CFString)
        guard result == .success else {
            throw WindowError.clickFailed
        }
    }

    /// Set checkbox to a specific value (checked/unchecked)
    /// - Parameters:
    ///   - appName: Name of the app (empty string for frontmost app)
    ///   - windowName: Name of the window (empty string for frontmost window)
    ///   - checkboxName: Name of the checkbox
    ///   - value: 0 for unchecked, 1 for checked
    static func setCheckboxValue(appName: String, windowName: String, checkboxName: String, value: Int) throws {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        let checkbox = try findCheckbox(in: window, checkboxName: checkboxName)

        // Set the AXValue attribute
        let cfValue = value as CFNumber
        let result = AXUIElementSetAttributeValue(checkbox, kAXValueAttribute as CFString, cfValue)
        guard result == .success else {
            throw WindowError.clickFailed
        }
    }

    /// Get list of button names in a window
    static func getWindowButtons(appName: String, windowName: String) throws -> [String] {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        return getAllButtons(in: window)
    }

    /// Get items from a popup menu
    /// - Parameters:
    ///   - appName: Name of the app (empty string for frontmost app)
    ///   - windowName: Name of the window (empty string for frontmost window)
    ///   - popupName: Name of the popup button
    /// - Returns: Array of menu item titles
    static func getPopupMenuItems(appName: String, windowName: String, popupName: String) throws -> [String] {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)

        // Find the popup button
        guard let popup = findElement(in: window, role: kAXPopUpButtonRole as String, name: popupName) else {
            throw WindowError.buttonNotFound(popupName)
        }

        // Press the popup to open the menu
        _ = AXUIElementPerformAction(popup, kAXPressAction as CFString)
        // Note: Some apps return error even though popup opens, so we ignore the result

        // Small delay for menu to appear (much shorter than Rust's 200ms)
        Thread.sleep(forTimeInterval: 0.05)  // 50ms

        var menuItems: [String] = []

        // Try to get menu via AXMenu attribute
        var menuRef: AnyObject?
        if AXUIElementCopyAttributeValue(popup, "AXMenu" as CFString, &menuRef) == .success,
           menuRef != nil {
            let menu = menuRef as! AXUIElement
            menuItems = getMenuItems(from: menu)
        } else {
            // Try to find menu in children
            var childrenRef: AnyObject?
            if AXUIElementCopyAttributeValue(popup, kAXChildrenAttribute as CFString, &childrenRef) == .success,
               childrenRef != nil {
                let childrenCF = childrenRef as! CFArray
                let count = CFArrayGetCount(childrenCF)

                for i in 0..<count {
                    let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)
                    var roleRef: AnyObject?
                    if AXUIElementCopyAttributeValue(child, kAXRoleAttribute as CFString, &roleRef) == .success,
                       let role = roleRef as? String,
                       role == kAXMenuRole as String {
                        menuItems = getMenuItems(from: child)
                        break
                    }
                }
            }
        }

        return menuItems
    }

    /// Get all text from a window
    /// - Parameters:
    ///   - appName: Name of the app (empty string for frontmost app)
    ///   - windowName: Name of the window (empty string for frontmost window)
    /// - Returns: Array of strings in format "[Role] Text"
    static func getWindowText(appName: String, windowName: String) throws -> [String] {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        var textStrings: [String] = []
        collectText(from: window, into: &textStrings)
        return textStrings
    }

    // MARK: - Helper Methods

    private static func getApp(appName: String) throws -> NSRunningApplication {
        if appName.isEmpty {
            guard let app = NSWorkspace.shared.frontmostApplication else {
                throw WindowError.noFrontmostApp
            }
            return app
        } else {
            let runningApps = NSWorkspace.shared.runningApplications
            guard let app = runningApps.first(where: { $0.localizedName == appName }) else {
                throw WindowError.appNotFound(appName)
            }
            return app
        }
    }

    private static func getWindow(app: NSRunningApplication, windowName: String) throws -> AXUIElement {
        let appElement = AXUIElementCreateApplication(app.processIdentifier)

        // Get all windows
        var windowsRef: AnyObject?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              windowsRef != nil else {
            throw WindowError.windowNotFound(windowName.isEmpty ? "<focused>" : windowName)
        }

        let windowsCF = windowsRef as! CFArray
        let count = CFArrayGetCount(windowsCF)

        if windowName.isEmpty {
            // Return the first (frontmost) window
            guard count > 0 else {
                throw WindowError.windowNotFound("<focused>")
            }
            return unsafeBitCast(CFArrayGetValueAtIndex(windowsCF, 0), to: AXUIElement.self)
        } else {
            // Find window by name
            for i in 0..<count {
                let window = unsafeBitCast(CFArrayGetValueAtIndex(windowsCF, i), to: AXUIElement.self)
                var titleRef: AnyObject?
                if AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleRef) == .success,
                   let title = titleRef as? String,
                   softMatch(title, windowName) {
                    return window
                }
            }
            throw WindowError.windowNotFound(windowName)
        }
    }

    private static func findButton(in window: AXUIElement, buttonName: String) throws -> AXUIElement {
        if let button = findElement(in: window, role: kAXButtonRole as String, name: buttonName) {
            return button
        }
        throw WindowError.buttonNotFound(buttonName)
    }

    private static func findCheckbox(in window: AXUIElement, checkboxName: String) throws -> AXUIElement {
        if let checkbox = findElement(in: window, role: kAXCheckBoxRole as String, name: checkboxName) {
            return checkbox
        }
        throw WindowError.checkboxNotFound(checkboxName)
    }

    private static func getAllButtons(in window: AXUIElement) -> [String] {
        var buttons: [String] = []
        collectElements(in: window, role: kAXButtonRole as String, into: &buttons)
        return buttons
    }

    /// Recursively find an element by role and name
    private static func findElement(in element: AXUIElement, role: String, name: String) -> AXUIElement? {
        // Check if this element matches
        var roleRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleRef) == .success,
           let elementRole = roleRef as? String,
           elementRole == role {
            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef) == .success,
               let title = titleRef as? String,
               softMatch(title, name) {
                return element
            }
        }

        // Recursively search children
        var childrenRef: AnyObject?
        guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
              childrenRef != nil else {
            return nil
        }

        let childrenCF = childrenRef as! CFArray
        let count = CFArrayGetCount(childrenCF)

        for i in 0..<count {
            let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)
            if let found = findElement(in: child, role: role, name: name) {
                return found
            }
        }

        return nil
    }

    /// Recursively collect element names by role
    private static func collectElements(in element: AXUIElement, role: String, into results: inout [String]) {
        // Check if this element matches the role
        var roleRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleRef) == .success,
           let elementRole = roleRef as? String,
           elementRole == role {
            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef) == .success,
               let title = titleRef as? String,
               !title.isEmpty {
                results.append(title)
            }
        }

        // Recursively search children
        var childrenRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
           childrenRef != nil {
            let childrenCF = childrenRef as! CFArray
            let count = CFArrayGetCount(childrenCF)

            for i in 0..<count {
                let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)
                collectElements(in: child, role: role, into: &results)
            }
        }
    }

    /// Soft match (case-insensitive, whitespace-insensitive, partial matching)
    private static func softMatch(_ haystack: String, _ needle: String) -> Bool {
        let normalizedHaystack = haystack.lowercased().filter { !$0.isWhitespace }
        let normalizedNeedle = needle.lowercased().filter { !$0.isWhitespace }
        return normalizedHaystack == normalizedNeedle || normalizedHaystack.contains(normalizedNeedle)
    }

    /// Get menu item titles from a menu element
    private static func getMenuItems(from menu: AXUIElement) -> [String] {
        var items: [String] = []

        var childrenRef: AnyObject?
        guard AXUIElementCopyAttributeValue(menu, kAXChildrenAttribute as CFString, &childrenRef) == .success,
              childrenRef != nil else {
            return items
        }

        let childrenCF = childrenRef as! CFArray
        let count = CFArrayGetCount(childrenCF)

        for i in 0..<count {
            let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)
            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef) == .success,
               let title = titleRef as? String,
               !title.isEmpty {
                items.append(title)
            }
        }

        return items
    }

    /// Recursively collect all text from an element
    private static func collectText(from element: AXUIElement, into results: inout [String]) {
        // Get role
        var roleRef: AnyObject?
        let role = (AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleRef) == .success && roleRef != nil)
            ? (roleRef as? String ?? "Unknown")
            : "Unknown"

        // Try to get AXValue
        var valueRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXValueAttribute as CFString, &valueRef) == .success,
           valueRef != nil,
           let text = valueRef as? String,
           !text.isEmpty {
            results.append("[\(role)] \(text)")
        }

        // Try AXTitle
        var titleRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef) == .success,
           let title = titleRef as? String,
           !title.isEmpty,
           !results.contains(where: { $0.contains(title) }) {
            results.append("[\(role) Title] \(title)")
        }

        // Try AXDescription
        var descRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXDescriptionAttribute as CFString, &descRef) == .success,
           let desc = descRef as? String,
           !desc.isEmpty,
           !results.contains(where: { $0.contains(desc) }) {
            results.append("[\(role) Description] \(desc)")
        }

        // Recursively search children
        var childrenRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef) == .success,
           childrenRef != nil {
            let childrenCF = childrenRef as! CFArray
            let count = CFArrayGetCount(childrenCF)

            for i in 0..<count {
                let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenCF, i), to: AXUIElement.self)
                collectText(from: child, into: &results)
            }
        }
    }

    // MARK: - Consolidated Window Operations

    /// Check if a window exists
    static func windowExists(appName: String, windowName: String) -> Bool {
        do {
            let app = try getApp(appName: appName)
            _ = try getWindow(app: app, windowName: windowName)
            return true
        } catch {
            return false
        }
    }

    /// Get all window titles for an application
    static func getWindowTitles(appName: String) -> [String] {
        guard let app = (appName.isEmpty
            ? NSWorkspace.shared.frontmostApplication
            : NSWorkspace.shared.runningApplications.first {
                softMatch($0.localizedName ?? "", appName)
            })
        else {
            return []
        }

        let pid = app.processIdentifier
        let appElement = AXUIElementCreateApplication(pid)

        var windowsRef: AnyObject?
        guard AXUIElementCopyAttributeValue(appElement, kAXWindowsAttribute as CFString, &windowsRef) == .success,
              windowsRef != nil else {
            return []
        }

        let windowsCF = windowsRef as! CFArray
        let count = CFArrayGetCount(windowsCF)
        var titles: [String] = []

        for i in 0..<count {
            let window = unsafeBitCast(CFArrayGetValueAtIndex(windowsCF, i), to: AXUIElement.self)
            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleRef) == .success,
               let title = titleRef as? String {
                titles.append(title)
            }
        }

        return titles
    }

    /// Wait for a window to meet a specific condition
    /// - Parameters:
    ///   - appName: Name of the app (empty for frontmost)
    ///   - windowName: Name of the window (empty for frontmost)
    ///   - condition: Condition to wait for
    ///   - timeout: Maximum time to wait in milliseconds
    /// - Returns: true if condition was met, false if timeout
    static func waitForWindow(
        appName: String,
        windowName: String,
        condition: WindowCondition,
        timeout: Int
    ) -> Bool {
        let startTime = Date()
        let timeoutSeconds = Double(timeout) / 1000.0
        let pollInterval = 0.05  // 50ms

        while Date().timeIntervalSince(startTime) < timeoutSeconds {
            let conditionMet: Bool

            switch condition {
            case .exists:
                conditionMet = windowExists(appName: appName, windowName: windowName)

            case .closed:
                conditionMet = !windowExists(appName: appName, windowName: windowName)

            case .focused:
                if let info = try? AppOps.getFrontmostInfo() {
                    let appMatches = appName.isEmpty || softMatch(info.appName, appName)
                    let windowMatches = windowName.isEmpty || softMatch(info.windowName, windowName)
                    conditionMet = appMatches && windowMatches
                } else {
                    conditionMet = false
                }
            }

            if conditionMet {
                return true
            }

            Thread.sleep(forTimeInterval: pollInterval)
        }

        return false
    }

    /// Close a window
    /// - Parameters:
    ///   - appName: Name of the app (empty for frontmost)
    ///   - windowName: Name of the window (empty for frontmost)
    ///   - retryTimeout: If provided, retry closing until window is gone or timeout (in milliseconds)
    static func closeWindow(appName: String, windowName: String, retryTimeout: Int?) throws {
        let closeOnce = {
            let app = try self.getApp(appName: appName)
            let window = try self.getWindow(app: app, windowName: windowName)

            // Get close button
            var closeButtonRef: AnyObject?
            guard AXUIElementCopyAttributeValue(window, kAXCloseButtonAttribute as CFString, &closeButtonRef) == .success,
                  closeButtonRef != nil else {
                throw WindowError.closeFailed
            }

            let closeButton = closeButtonRef as! AXUIElement
            let result = AXUIElementPerformAction(closeButton, kAXPressAction as CFString)

            guard result == .success else {
                throw WindowError.closeFailed
            }
        }

        // If no retry, just close once
        guard let timeout = retryTimeout else {
            try closeOnce()
            return
        }

        // Retry until closed or timeout
        let startTime = Date()
        let timeoutSeconds = Double(timeout) / 1000.0

        while Date().timeIntervalSince(startTime) < timeoutSeconds {
            // Check if already closed
            if !windowExists(appName: appName, windowName: windowName) {
                return
            }

            // Try to close
            try? closeOnce()

            // Wait a bit
            Thread.sleep(forTimeInterval: 0.1)
        }

        // Final check
        if !windowExists(appName: appName, windowName: windowName) {
            return
        }

        throw WindowError.timeout
    }
}
