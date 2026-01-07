import Cocoa
import ApplicationServices

enum KeystrokeError: Error {
    case noFrontmostApp
    case appNotFound(String)
    case invalidModifiers
}

class Keystroke {
    /// Send a keystroke to an application
    /// modifiers: bit flags (shift=1, control=2, option=4, command=8)
    static func send(appName: String, keyChar: String, modifiers: Int) throws {
        guard let keyCode = keyCharToKeyCode(keyChar) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Invalid key character: \(keyChar)"])
        }

        let app: NSRunningApplication

        if appName.isEmpty {
            guard let frontmost = NSWorkspace.shared.frontmostApplication else {
                throw KeystrokeError.noFrontmostApp
            }
            app = frontmost
        } else {
            let runningApps = NSWorkspace.shared.runningApplications
            guard let foundApp = runningApps.first(where: { $0.localizedName == appName }) else {
                throw KeystrokeError.appNotFound(appName)
            }
            app = foundApp
        }

        // Activate the app first
        app.activate(options: [])

        // Small delay to ensure app is active
        Thread.sleep(forTimeInterval: 0.05)

        // Convert modifiers
        var cgFlags: CGEventFlags = []
        if (modifiers & 1) != 0 { cgFlags.insert(.maskShift) }
        if (modifiers & 2) != 0 { cgFlags.insert(.maskControl) }
        if (modifiers & 4) != 0 { cgFlags.insert(.maskAlternate) }
        if (modifiers & 8) != 0 { cgFlags.insert(.maskCommand) }

        // Create and post key down event
        guard let keyDown = CGEvent(keyboardEventSource: nil, virtualKey: keyCode, keyDown: true) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create key down event"])
        }
        keyDown.flags = cgFlags
        keyDown.post(tap: .cghidEventTap)

        // Create and post key up event
        guard let keyUp = CGEvent(keyboardEventSource: nil, virtualKey: keyCode, keyDown: false) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create key up event"])
        }
        keyUp.flags = cgFlags
        keyUp.post(tap: .cghidEventTap)
    }

    /// Map a character to its CGKeyCode
    private static func keyCharToKeyCode(_ char: String) -> CGKeyCode? {
        let lower = char.lowercased()

        // Letters
        let letterMap: [String: CGKeyCode] = [
            "a": 0x00, "b": 0x0B, "c": 0x08, "d": 0x02, "e": 0x0E, "f": 0x03,
            "g": 0x05, "h": 0x04, "i": 0x22, "j": 0x26, "k": 0x28, "l": 0x25,
            "m": 0x2E, "n": 0x2D, "o": 0x1F, "p": 0x23, "q": 0x0C, "r": 0x0F,
            "s": 0x01, "t": 0x11, "u": 0x20, "v": 0x09, "w": 0x0D, "x": 0x07,
            "y": 0x10, "z": 0x06
        ]

        // Numbers
        let numberMap: [String: CGKeyCode] = [
            "0": 0x1D, "1": 0x12, "2": 0x13, "3": 0x14, "4": 0x15,
            "5": 0x17, "6": 0x16, "7": 0x1A, "8": 0x1C, "9": 0x19
        ]

        // Special keys
        let specialMap: [String: CGKeyCode] = [
            " ": 0x31,      // Space
            "\n": 0x24,     // Return
            "\t": 0x30,     // Tab
            "\u{8}": 0x33,  // Delete/Backspace
            "\u{1B}": 0x35, // Escape
            "`": 0x32, "-": 0x1B, "=": 0x18,
            "[": 0x21, "]": 0x1E, "\\": 0x2A,
            ";": 0x29, "'": 0x27, ",": 0x2B,
            ".": 0x2F, "/": 0x2C
        ]

        // Function keys
        let functionMap: [String: CGKeyCode] = [
            "f1": 0x7A, "f2": 0x78, "f3": 0x63, "f4": 0x76,
            "f5": 0x60, "f6": 0x61, "f7": 0x62, "f8": 0x64,
            "f9": 0x65, "f10": 0x6D, "f11": 0x67, "f12": 0x6F
        ]

        // Arrow keys
        let arrowMap: [String: CGKeyCode] = [
            "left": 0x7B, "right": 0x7C, "down": 0x7D, "up": 0x7E
        ]

        return letterMap[lower]
            ?? numberMap[lower]
            ?? specialMap[lower]
            ?? functionMap[lower]
            ?? arrowMap[lower]
    }
}
