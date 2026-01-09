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

        let pid = app.processIdentifier
        let appElement = AXUIElementCreateApplication(pid)

var focusedWindowRef: AnyObject?
if AXUIElementCopyAttributeValue(appElement, kAXFocusedWindowAttribute as CFString, &focusedWindowRef) == .success,
   focusedWindowRef != nil {
    let window = focusedWindowRef! as! AXUIElement  // <--- force cast
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
        NSWorkspace.shared.runningApplications
            .compactMap { $0.localizedName }
            .filter { !$0.isEmpty }
    }

    /// Focus (activate) an application
    static func focusApp(
        appName: String,
        windowName: String = "",
        shouldSwitch: Bool = true,
        shouldLaunch: Bool = false,
        timeout: Int = 1000
    ) throws {
        if appName.isEmpty { return }

        var app = NSWorkspace.shared.runningApplications.first {
            $0.localizedName == appName && $0.activationPolicy == .regular
        }

        if app == nil && shouldLaunch {
            guard let appURL = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID(forAppName: appName)) else {
                throw AppError.appNotFound(appName)
            }

            let config = NSWorkspace.OpenConfiguration()
            config.activates = false // activate later
            NSWorkspace.shared.openApplication(at: appURL, configuration: config)
            Thread.sleep(forTimeInterval: 0.5)

            app = NSWorkspace.shared.runningApplications.first {
                $0.localizedName == appName && $0.activationPolicy == .regular
            }
        }

        guard let targetApp = app else {
            throw AppError.appNotFound(appName)
        }

        if shouldSwitch {
            let success = targetApp.activate(options: [.activateIgnoringOtherApps])
            if !success {
                throw AppError.activationFailed
            }
        }

        if !windowName.isEmpty && timeout > 0 {
            let startTime = Date()
            let timeoutSeconds = Double(timeout) / 1000.0

            while Date().timeIntervalSince(startTime) < timeoutSeconds {
                if let info = try? getFrontmostInfo(),
                   info.appName == appName,
                   info.windowName == windowName {
                    return
                }
                Thread.sleep(forTimeInterval: 0.05)
            }
        }
    }

    /// Launch an application by name
    static func launchApp(appName: String) throws {
        guard let appURL = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleID(forAppName: appName)) else {
            throw AppError.appNotFound(appName)
        }

        let config = NSWorkspace.OpenConfiguration()
        config.activates = true  // launch and bring to front
        NSWorkspace.shared.openApplication(at: appURL, configuration: config)
    }

    /// Map human-readable app name to bundle ID
    private static func bundleID(forAppName name: String) -> String {
        switch name {
        case "TextEdit": return "com.apple.TextEdit"
        case "Finder": return "com.apple.finder"
        case "Soundminer_Intel": return "com.soundminer.Intel"
        case "Soundminer_AppleSilicon": return "com.soundminer.AppleSilicon"
        default: return name // fallback: assume user passed a bundle ID
        }
    }

    /// Check if the currently focused UI element is a text input field
    static func isInTextField() -> Bool {
        let systemWide = AXUIElementCreateSystemWide()

var focusedElementRef: AnyObject?
let result = AXUIElementCopyAttributeValue(
    systemWide,
    kAXFocusedUIElementAttribute as CFString,
    &focusedElementRef
)

guard result == .success else { return false }
let focusedElement = focusedElementRef as! AXUIElement

        var roleRef: AnyObject?
        let roleResult = AXUIElementCopyAttributeValue(
            focusedElement,
            kAXRoleAttribute as CFString,
            &roleRef
        )

        guard roleResult == .success, let role = roleRef as? String else {
            return false
        }

        return role == kAXTextFieldRole as String
            || role == kAXTextAreaRole as String
            || role == kAXComboBoxRole as String
            || role == "AXSearchField"
    }
}
