import Cocoa
import ApplicationServices

/// Represents a menu item with all its properties
struct MenuItem: Codable {
    let title: String
    let path: [String]  // Full path to this item (e.g., ["File", "Save"])
    var enabled: Bool
    var checked: Bool
    let cmdChar: String?
    let cmdModifiers: Int?
    let children: [MenuItem]?

    // Internal data not serialized to JSON (not part of Codable)
    private var pid: pid_t = 0  // Default value for Codable compliance

    enum CodingKeys: String, CodingKey {
        case title, path, enabled, checked, cmdChar, cmdModifiers, children
    }

    /// Create a new MenuItem from AXUIElement
    init(element: AXUIElement, pid: pid_t, path: [String]) {
        self.pid = pid
        self.path = path

        // Get title
        var titleRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXTitleAttribute as CFString, &titleRef)
        self.title = (titleRef as? String) ?? "(no title)"

        // Get enabled state
        var enabledRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXEnabledAttribute as CFString, &enabledRef) == .success {
            self.enabled = (enabledRef as? Bool) ?? true
        } else {
            self.enabled = true
        }

        // Get checked state (mark character)
        var markCharRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXMenuItemMarkCharAttribute as CFString, &markCharRef) == .success,
           let markChar = markCharRef as? String, !markChar.isEmpty {
            self.checked = true
        } else {
            self.checked = false
        }

        // Get command key shortcut
        var cmdCharRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXMenuItemCmdCharAttribute as CFString, &cmdCharRef) == .success {
            self.cmdChar = cmdCharRef as? String
        } else {
            self.cmdChar = nil
        }

        // Get command key modifiers
        var cmdModsRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXMenuItemCmdModifiersAttribute as CFString, &cmdModsRef) == .success {
            self.cmdModifiers = cmdModsRef as? Int
        } else {
            self.cmdModifiers = nil
        }

        // Try to get command ID (for Apple Events) - this may not be available for all apps
        // We'll log what attributes are actually available for debugging
        var attributesRef: CFArray?
        if AXUIElementCopyAttributeNames(element, &attributesRef) == .success,
           let attrsCF = attributesRef {
            let count = CFArrayGetCount(attrsCF)
            for i in 0..<count {
                if let attr = unsafeBitCast(CFArrayGetValueAtIndex(attrsCF, i), to: CFString.self) as String? {
                    if attr.lowercased().contains("command") || attr.lowercased().contains("action") {
                        NSLog("MenuItem '\(title)' has attribute: \(attr)")
                    }
                }
            }
        }

        // Get children (submenu items)
        var childrenRef: AnyObject?
        AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &childrenRef)

        if let childrenCF = childrenRef {
            let childrenArray = childrenCF as! CFArray
            let count = CFArrayGetCount(childrenArray)
            if count > 0 {
                var childItems: [MenuItem] = []

                // Special case: if there's only one child with no title, it's likely an AXMenu container
                // Skip it and use its children instead
                if count == 1 {
                    let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, 0), to: AXUIElement.self)
                    var titleRef: AnyObject?
                    AXUIElementCopyAttributeValue(child, kAXTitleAttribute as CFString, &titleRef)
                    let childTitle = (titleRef as? String) ?? ""

                    // If the single child has no title or "(no title)", skip it and use its children
                    if childTitle.isEmpty || childTitle == "(no title)" {
                        var grandchildrenRef: AnyObject?
                        if AXUIElementCopyAttributeValue(child, kAXChildrenAttribute as CFString, &grandchildrenRef) == .success,
                           grandchildrenRef != nil {
                            let grandchildrenArray = grandchildrenRef as! CFArray
                            let grandCount = CFArrayGetCount(grandchildrenArray)
                            for i in 0..<grandCount {
                                let grandchild = unsafeBitCast(CFArrayGetValueAtIndex(grandchildrenArray, i), to: AXUIElement.self)
                                var childPath = path
                                childPath.append(self.title)
                                childItems.append(MenuItem(element: grandchild, pid: pid, path: childPath))
                            }
                            self.children = childItems.isEmpty ? nil : childItems
                            return
                        }
                    }
                }

                // Normal case: process children directly
                for i in 0..<count {
                    let child = unsafeBitCast(CFArrayGetValueAtIndex(childrenArray, i), to: AXUIElement.self)
                    var childPath = path
                    childPath.append(self.title)
                    childItems.append(MenuItem(element: child, pid: pid, path: childPath))
                }
                self.children = childItems
            } else {
                self.children = nil
            }
        } else {
            self.children = nil
        }
    }

    /// Update the state of this menu item (enabled, checked)
    mutating func update(from element: AXUIElement) {
        // Update enabled state
        var enabledRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXEnabledAttribute as CFString, &enabledRef) == .success {
            self.enabled = (enabledRef as? Bool) ?? true
        }

        // Update checked state
        var markCharRef: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXMenuItemMarkCharAttribute as CFString, &markCharRef) == .success,
           let markChar = markCharRef as? String, !markChar.isEmpty {
            self.checked = true
        } else {
            self.checked = false
        }
    }

    /// Execute this menu item using AXPress (direct execution, no UI interaction)
    func execute() throws {
        try executeViaAccessibility()
    }

    /// Execute by traversing accessibility hierarchy and using AXPress
    /// Note: This is currently unused - MenuOps.menuClick is the primary execution path
    /// Keeping this for potential future use or as a reference implementation
    private func executeViaAccessibility() throws {
        // This method is not currently used because menu execution goes through
        // MenuOps.menuClick which has the optimized container-skipping logic
        throw NSError(domain: "MenuItem", code: -1,
                     userInfo: [NSLocalizedDescriptionKey: "Use MenuOps.menuClick instead"])
    }
}
