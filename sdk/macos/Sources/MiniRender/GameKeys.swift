import AppKit

/// Mac 键盘 → DOM 的 `key` / `code`（`wx.onKeyDown` / `onKeyUp`，PC 小游戏按它们判方向键、WASD、空格）。
/// `code` 按**物理键位**（keyCode），与输入法 / 键盘布局无关；`key` 取实际字符（布局相关），功能键给 DOM 的名字。
enum GameKeys {
    /// keyCode → (key, code)：功能键与不随布局变的键
    private static let named: [UInt16: (String, String)] = [
        36: ("Enter", "Enter"), 76: ("Enter", "NumpadEnter"), 48: ("Tab", "Tab"), 49: (" ", "Space"),
        51: ("Backspace", "Backspace"), 117: ("Delete", "Delete"), 53: ("Escape", "Escape"),
        123: ("ArrowLeft", "ArrowLeft"), 124: ("ArrowRight", "ArrowRight"), 125: ("ArrowDown", "ArrowDown"), 126: ("ArrowUp", "ArrowUp"),
        115: ("Home", "Home"), 119: ("End", "End"), 116: ("PageUp", "PageUp"), 121: ("PageDown", "PageDown"),
        122: ("F1", "F1"), 120: ("F2", "F2"), 99: ("F3", "F3"), 118: ("F4", "F4"), 96: ("F5", "F5"), 97: ("F6", "F6"),
        98: ("F7", "F7"), 100: ("F8", "F8"), 101: ("F9", "F9"), 109: ("F10", "F10"), 103: ("F11", "F11"), 111: ("F12", "F12"),
    ]

    /// ANSI 键位的字母 / 数字 / 符号键 → code
    private static let codes: [UInt16: String] = [
        0: "KeyA", 11: "KeyB", 8: "KeyC", 2: "KeyD", 14: "KeyE", 3: "KeyF", 5: "KeyG", 4: "KeyH", 34: "KeyI", 38: "KeyJ",
        40: "KeyK", 37: "KeyL", 46: "KeyM", 45: "KeyN", 31: "KeyO", 35: "KeyP", 12: "KeyQ", 15: "KeyR", 1: "KeyS", 17: "KeyT",
        32: "KeyU", 9: "KeyV", 13: "KeyW", 7: "KeyX", 16: "KeyY", 6: "KeyZ",
        29: "Digit0", 18: "Digit1", 19: "Digit2", 20: "Digit3", 21: "Digit4", 23: "Digit5", 22: "Digit6", 26: "Digit7", 28: "Digit8", 25: "Digit9",
        27: "Minus", 24: "Equal", 33: "BracketLeft", 30: "BracketRight", 42: "Backslash", 41: "Semicolon", 39: "Quote",
        43: "Comma", 47: "Period", 44: "Slash", 50: "Backquote",
    ]

    static func name(_ e: NSEvent) -> (key: String, code: String) {
        if let n = named[e.keyCode] { return n }
        let ch = e.characters ?? ""
        return (ch.isEmpty ? "Unidentified" : ch, codes[e.keyCode] ?? "Unidentified")
    }

    /// 修饰键（flagsChanged）：(key, code, 对应的标志位)
    static func modifier(_ keyCode: UInt16) -> (String, String, NSEvent.ModifierFlags)? {
        switch keyCode {
        case 56: return ("Shift", "ShiftLeft", .shift)
        case 60: return ("Shift", "ShiftRight", .shift)
        case 59: return ("Control", "ControlLeft", .control)
        case 62: return ("Control", "ControlRight", .control)
        case 58: return ("Alt", "AltLeft", .option)
        case 61: return ("Alt", "AltRight", .option)
        case 55: return ("Meta", "MetaLeft", .command)
        case 54: return ("Meta", "MetaRight", .command)
        case 57: return ("CapsLock", "CapsLock", .capsLock)
        default: return nil
        }
    }
}
