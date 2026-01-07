import Cocoa
import ApplicationServices

enum AppError: Error {
    case appNotFound(String)
    case noFrontmostApp
    case activationFailed
    case launchFailed
}

/// Options for focusing an app
struct AppFocusOptions {
    let windowName: String
    let shouldSwitch: Bool
    let shouldLaunch: Bool
    let timeout: Int  // milliseconds

    static let `default` = AppFocusOptions(
        windowName: "",
        shouldSwitch: true,
        shouldLaunch: false,
        timeout: 1000
    )
}

/// Information about the frontmost application
struct FrontmostInfo {
    let appName: String
    let windowName: String
}

class AppOps {

    /// Get information about the frontmost application and window
    static func getFrontmostInfo() throws -> FrontmostInfo {
        guard let app = NSWorkspace.shared.frontmostApplication else {
            throw AppError.noFrontmostApp
        }

        let appName = app.localizedName ?? ""
        var windowName = ""

        // Get frontmost window title using AX API
        let pid = app.processIdentifier
        let appElement = AXUIElementCreateApplication(pid)

        var focusedWindowRef: AnyObject?
        if AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focusedWindowRef) == .success,
           focusedWindowRef != nil {
            let window = focusedWindowRef as! AXUIElement
            var titleRef: AnyObject?
            if AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &titleRef) == .success,
               let title = titleRef as? String {
                windowName = title
            }
        }

        return FrontmostInfo(appName: appName, windowName: windowName)
    }

    /// Get list of all running application names
    static func getRunningApps() -> [String] {
        return NSWorkspace.shared.runningApplications
            .compactMap { $0.localizedName }
            .filter { !$0.isEmpty }
    }

    /// Focus (activate) an application
    /// - Parameters:
    ///   - appName: Name of app to focus (empty string = frontmost app)
    ///   - windowName: Name of specific window to focus (empty = any window)
    ///   - shouldSwitch: Whether to switch to the app
    ///   - shouldLaunch: Whether to launch if not running
    ///   - timeout: Maximum time to wait in milliseconds
    static func focusApp(
        appName: String,
        windowName: String = "",
        shouldSwitch: Bool = true,
        shouldLaunch: Bool = false,
        timeout: Int = 1000
    ) throws {
        // If appName is empty, nothing to do (already frontmost)
        if appName.isEmpty {
            return
        }

        // Find the app
        let runningApps = NSWorkspace.shared.runningApplications
        var app = runningApps.first { softMatch($0.localizedName ?? "", appName) }

        // Launch if needed and not found
        if app == nil && shouldLaunch {
            if NSWorkspace.shared.launchApplication(appName) {
                // Wait a bit for app to launch
                Thread.sleep(forTimeInterval: 0.5)
                app = NSWorkspace.shared.runningApplications.first {
                    softMatch($0.localizedName ?? "", appName)
                }
            }
        }

        guard let targetApp = app else {
            throw AppError.appNotFound(appName)
        }

        // Activate the app if shouldSwitch
        if shouldSwitch {
            let success = targetApp.activate(options: [.activateIgnoringOtherApps])
            if !success {
                throw AppError.activationFailed
            }
        }

        // If specific window requested, wait for it to be focused
        if !windowName.isEmpty && timeout > 0 {
            let startTime = Date()
            let timeoutSeconds = Double(timeout) / 1000.0

            while Date().timeIntervalSince(startTime) < timeoutSeconds {
                if let info = try? getFrontmostInfo(),
                   softMatch(info.appName, appName),
                   softMatch(info.windowName, windowName) {
                    return
                }
                Thread.sleep(forTimeInterval: 0.05)
            }
        }
    }

    /// Launch an application
    static func launchApp(appName: String) throws {
        let success = NSWorkspace.shared.launchApplication(appName)
        if !success {
            throw AppError.launchFailed
        }
    }

    /// Soft match (case-insensitive, whitespace-insensitive, partial matching)
    private static func softMatch(_ haystack: String, _ needle: String) -> Bool {
        let normalizedHaystack = haystack.lowercased().filter { !$0.isWhitespace }
        let normalizedNeedle = needle.lowercased().filter { !$0.isWhitespace }
        return normalizedHaystack == normalizedNeedle || normalizedHaystack.contains(normalizedNeedle)
    }

    /// Check if the currently focused UI element is a text input field
    ///
    /// Returns true if focused element is a text field, text area, combo box, or search field
    static func isInTextField() -> Bool {
        // Get system-wide focused element
        let systemWide = AXUIElementCreateSystemWide()

        var focusedElementRef: AnyObject?
        let result = AXUIElementCopyAttributeValue(
            systemWide,
            kAXFocusedUIElementAttribute as CFString,
            &focusedElementRef
        )

        // If no element is focused or error getting it, assume not in text field
        guard result == .success, let focusedElement = focusedElementRef else {
            return false
        }

        // Get the role of the focused element
        var roleRef: AnyObject?
        let roleResult = AXUIElementCopyAttributeValue(
            focusedElement as! AXUIElement,
            kAXRoleAttribute as CFString,
            &roleRef
        )

        guard roleResult == .success, let role = roleRef as? String else {
            return false
        }

        // Check if the role indicates a text input field
        return role == kAXTextFieldRole as String
            || role == kAXTextAreaRole as String
            || role == kAXComboBoxRole as String
            || role == "AXSearchField"  // kAXSearchFieldRole doesn't exist, use string
    }
}
