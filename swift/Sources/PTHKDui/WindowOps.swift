import Cocoa
import ApplicationServices

enum WindowError: Error {
    case noFrontmostApp
    case appNotFound(String)
    case windowNotFound(String)
    case buttonNotFound(String)
    case checkboxNotFound(String)
    case clickFailed
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

    /// Get list of button names in a window
    static func getWindowButtons(appName: String, windowName: String) throws -> [String] {
        let app = try getApp(appName: appName)
        let window = try getWindow(app: app, windowName: windowName)
        return getAllButtons(in: window)
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
}
