import Cocoa
import ApplicationServices

enum KeystrokeError: Error {
    case noFrontmostApp
    case appNotFound(String)
    case invalidModifiers
}

class Keystroke {
    // Event marker to prevent event tap from catching our own events (matches Rust)
    private static let APP_EVENT_MARKER: Int64 = 0x5054484B44 // "PTHKD"
    private static let EVENT_USER_DATA_FIELD: CGEventField = CGEventField(rawValue: 127)!

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

    /// Send a global keystroke with multiple keys and modifiers
    /// - Parameters:
    ///   - keyCodes: Array of key codes to press
    ///   - modifierFlags: CGEventFlags for modifiers
    static func sendGlobalKeystroke(keyCodes: [CGKeyCode], modifierFlags: CGEventFlags) throws {
        guard let eventSource = CGEventSource(stateID: .hidSystemState) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create event source"])
        }

        // Send key-down events with modifier flags
        for keyCode in keyCodes {
            guard let keyDown = CGEvent(keyboardEventSource: eventSource, virtualKey: keyCode, keyDown: true) else {
                throw NSError(domain: "Keystroke", code: -1,
                             userInfo: [NSLocalizedDescriptionKey: "Failed to create key down event for keycode \(keyCode)"])
            }

            // Mark event to prevent event tap from catching it
            keyDown.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)

            if modifierFlags != [] {
                keyDown.flags = modifierFlags
            }

            keyDown.post(tap: .cghidEventTap)
        }

        // Send key-up events in reverse order
        for keyCode in keyCodes.reversed() {
            guard let keyUp = CGEvent(keyboardEventSource: eventSource, virtualKey: keyCode, keyDown: false) else {
                throw NSError(domain: "Keystroke", code: -1,
                             userInfo: [NSLocalizedDescriptionKey: "Failed to create key up event for keycode \(keyCode)"])
            }

            // Mark event to prevent event tap from catching it
            keyUp.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)

            if modifierFlags != [] {
                keyUp.flags = modifierFlags
            }

            keyUp.post(tap: .cghidEventTap)
        }
    }

    /// Type text character by character
    /// - Parameters:
    ///   - text: The text to type
    ///   - markEvents: Whether to mark events with APP_MARKER (true = prevent event tap catching, false = appear as user input)
    static func typeText(text: String, markEvents: Bool = true) throws {
        guard let eventSource = CGEventSource(stateID: .hidSystemState) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create event source"])
        }

        for ch in text {
            guard let (keyCode, needsShift) = charToKeyCode(ch) else {
                throw NSError(domain: "Keystroke", code: -1,
                             userInfo: [NSLocalizedDescriptionKey: "Unsupported character: '\(ch)'"])
            }

            // Send shift down if needed
            if needsShift {
                guard let shiftDown = CGEvent(keyboardEventSource: eventSource, virtualKey: 56, keyDown: true) else {
                    throw NSError(domain: "Keystroke", code: -1,
                                 userInfo: [NSLocalizedDescriptionKey: "Failed to create shift down event"])
                }
                if markEvents {
                    shiftDown.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)
                }
                shiftDown.post(tap: .cghidEventTap)
            }

            // Send key down
            guard let keyDown = CGEvent(keyboardEventSource: eventSource, virtualKey: keyCode, keyDown: true) else {
                throw NSError(domain: "Keystroke", code: -1,
                             userInfo: [NSLocalizedDescriptionKey: "Failed to create key down event"])
            }
            if markEvents {
                keyDown.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)
            }
            if needsShift {
                keyDown.flags = .maskShift
            }
            keyDown.post(tap: .cghidEventTap)

            // Send key up
            guard let keyUp = CGEvent(keyboardEventSource: eventSource, virtualKey: keyCode, keyDown: false) else {
                throw NSError(domain: "Keystroke", code: -1,
                             userInfo: [NSLocalizedDescriptionKey: "Failed to create key up event"])
            }
            if markEvents {
                keyUp.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)
            }
            if needsShift {
                keyUp.flags = .maskShift
            }
            keyUp.post(tap: .cghidEventTap)

            // Send shift up if needed
            if needsShift {
                guard let shiftUp = CGEvent(keyboardEventSource: eventSource, virtualKey: 56, keyDown: false) else {
                    throw NSError(domain: "Keystroke", code: -1,
                                 userInfo: [NSLocalizedDescriptionKey: "Failed to create shift up event"])
                }
                if markEvents {
                    shiftUp.setIntegerValueField(EVENT_USER_DATA_FIELD, value: APP_EVENT_MARKER)
                }
                shiftUp.post(tap: .cghidEventTap)
            }

            // Small delay between characters
            Thread.sleep(forTimeInterval: 0.005)  // 5ms
        }
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

    /// Map a character to its keycode and whether it needs shift
    private static func charToKeyCode(_ ch: Character) -> (CGKeyCode, Bool)? {
        let str = String(ch)

        // Lowercase letters - no shift
        if ch >= "a" && ch <= "z" {
            return (keyCharToKeyCode(str), false)
        }

        // Uppercase letters - need shift
        if ch >= "A" && ch <= "Z" {
            return (keyCharToKeyCode(str.lowercased()), true)
        }

        // Numbers - no shift
        if ch >= "0" && ch <= "9" {
            return (keyCharToKeyCode(str), false)
        }

        // Special characters that need shift
        let shiftMap: [Character: String] = [
            "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
            "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
            "_": "-", "+": "=", "{": "[", "}": "]", "|": "\\",
            ":": ";", "\"": "'", "<": ",", ">": ".", "?": "/",
            "~": "`"
        ]

        if let baseKey = shiftMap[ch], let keyCode = keyCharToKeyCode(baseKey) {
            return (keyCode, true)
        }

        // Special characters that don't need shift
        let specialChars = [" ", "\n", "\t", "`", "-", "=", "[", "]", "\\", ";", "'", ",", ".", "/"]
        if specialChars.contains(str), let keyCode = keyCharToKeyCode(str) {
            return (keyCode, false)
        }

        return nil
    }

    /// Paste text using clipboard and Cmd+V
    /// - Parameter text: The text to paste
    ///
    /// This is useful for password fields that may filter out programmatic keystrokes.
    /// Works by:
    /// 1. Saving current clipboard
    /// 2. Setting clipboard to the text
    /// 3. Sending Cmd+V
    /// 4. Restoring previous clipboard
    static func pasteText(text: String) throws {
        let pasteboard = NSPasteboard.general

        // Save current clipboard contents
        let savedItems = pasteboard.pasteboardItems

        // Clear and set new text
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        // Small delay to ensure clipboard is set
        Thread.sleep(forTimeInterval: 0.01)  // 10ms

        // Send Cmd+V (keycode 9 = 'v')
        guard let eventSource = CGEventSource(stateID: .hidSystemState) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create event source"])
        }

        // Create Cmd+V key down
        guard let keyDown = CGEvent(keyboardEventSource: eventSource, virtualKey: 9, keyDown: true) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create Cmd+V key down"])
        }
        keyDown.flags = .maskCommand
        keyDown.post(tap: .cghidEventTap)

        // Create Cmd+V key up
        guard let keyUp = CGEvent(keyboardEventSource: eventSource, virtualKey: 9, keyDown: false) else {
            throw NSError(domain: "Keystroke", code: -1,
                         userInfo: [NSLocalizedDescriptionKey: "Failed to create Cmd+V key up"])
        }
        keyUp.flags = .maskCommand
        keyUp.post(tap: .cghidEventTap)

        // Small delay before restoring clipboard
        Thread.sleep(forTimeInterval: 0.05)  // 50ms

        // Restore previous clipboard
        pasteboard.clearContents()
        if let saved = savedItems {
            pasteboard.writeObjects(saved)
        }
    }
}

