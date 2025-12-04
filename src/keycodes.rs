// macOS Virtual Key Codes
// Reference: https://developer.apple.com/documentation/appkit/nsevent/specialkey
//
// Note: Key codes represent physical keys, not symbols.
// Shifted symbols (like @, #, $, |, _, etc.) use the same key code
// as their unshifted counterpart, just with MOD_SHIFT added.

#![allow(dead_code)]

// ====== Modifier Flags ======
// These can be combined with bitwise OR (|)

pub const MOD_CMD: u64 = 1 << 20; // Command (⌘)
pub const MOD_SHIFT: u64 = 1 << 17; // Shift (⇧)
pub const MOD_OPTION: u64 = 1 << 19; // Option/Alt (⌥)
pub const MOD_CONTROL: u64 = 1 << 18; // Control (⌃)

// ====== Letter Keys ======

pub const KEY_A: u16 = 0;
pub const KEY_S: u16 = 1;
pub const KEY_D: u16 = 2;
pub const KEY_F: u16 = 3;
pub const KEY_H: u16 = 4;
pub const KEY_G: u16 = 5;
pub const KEY_Z: u16 = 6;
pub const KEY_X: u16 = 7;
pub const KEY_C: u16 = 8;
pub const KEY_V: u16 = 9;
pub const KEY_B: u16 = 11;
pub const KEY_Q: u16 = 12;
pub const KEY_W: u16 = 13;
pub const KEY_E: u16 = 14;
pub const KEY_R: u16 = 15;
pub const KEY_Y: u16 = 16;
pub const KEY_T: u16 = 17;
pub const KEY_O: u16 = 31;
pub const KEY_U: u16 = 32;
pub const KEY_I: u16 = 34;
pub const KEY_P: u16 = 35;
pub const KEY_L: u16 = 37;
pub const KEY_J: u16 = 38;
pub const KEY_K: u16 = 40;
pub const KEY_N: u16 = 45;
pub const KEY_M: u16 = 46;

// ====== Number Keys (Top Row) ======
// Note: Shift+Number produces @, #, $, %, ^, &, *, (, )

pub const KEY_1: u16 = 18;
pub const KEY_2: u16 = 19;
pub const KEY_3: u16 = 20;
pub const KEY_4: u16 = 21;
pub const KEY_6: u16 = 22;
pub const KEY_5: u16 = 23;
pub const KEY_EQUAL: u16 = 24; // = (Shift = +)
pub const KEY_9: u16 = 25;
pub const KEY_7: u16 = 26;
pub const KEY_MINUS: u16 = 27; // - (Shift = _)
pub const KEY_8: u16 = 28;
pub const KEY_0: u16 = 29;

// ====== Keypad Numbers ======

pub const KEYPAD_0: u16 = 82;
pub const KEYPAD_1: u16 = 83;
pub const KEYPAD_2: u16 = 84;
pub const KEYPAD_3: u16 = 85;
pub const KEYPAD_4: u16 = 86;
pub const KEYPAD_5: u16 = 87;
pub const KEYPAD_6: u16 = 88;
pub const KEYPAD_7: u16 = 89;
pub const KEYPAD_8: u16 = 91;
pub const KEYPAD_9: u16 = 92;

// ====== Keypad Operations ======

pub const KEYPAD_DECIMAL: u16 = 65;
pub const KEYPAD_MULTIPLY: u16 = 67;
pub const KEYPAD_PLUS: u16 = 69;
pub const KEYPAD_CLEAR: u16 = 71;
pub const KEYPAD_DIVIDE: u16 = 75;
pub const KEYPAD_ENTER: u16 = 76;
pub const KEYPAD_MINUS: u16 = 78;
pub const KEYPAD_EQUALS: u16 = 81;

// ====== Function Keys ======

pub const KEY_F1: u16 = 122;
pub const KEY_F2: u16 = 120;
pub const KEY_F3: u16 = 99;
pub const KEY_F4: u16 = 118;
pub const KEY_F5: u16 = 96;
pub const KEY_F6: u16 = 97;
pub const KEY_F7: u16 = 98;
pub const KEY_F8: u16 = 100;
pub const KEY_F9: u16 = 101;
pub const KEY_F10: u16 = 109;
pub const KEY_F11: u16 = 103;
pub const KEY_F12: u16 = 111;
pub const KEY_F13: u16 = 105;
pub const KEY_F14: u16 = 107;
pub const KEY_F15: u16 = 113;
pub const KEY_F16: u16 = 106;
pub const KEY_F17: u16 = 64;
pub const KEY_F18: u16 = 79;
pub const KEY_F19: u16 = 80;
pub const KEY_F20: u16 = 90;

// ====== Arrow Keys ======

pub const KEY_LEFT: u16 = 123;
pub const KEY_RIGHT: u16 = 124;
pub const KEY_DOWN: u16 = 125;
pub const KEY_UP: u16 = 126;

// ====== Special Keys ======

pub const KEY_RETURN: u16 = 36;
pub const KEY_TAB: u16 = 48;
pub const KEY_SPACE: u16 = 49;
pub const KEY_DELETE: u16 = 51; // Backspace
pub const KEY_ESCAPE: u16 = 53;
pub const KEY_FORWARD_DELETE: u16 = 117; // Del key
pub const KEY_HOME: u16 = 115;
pub const KEY_END: u16 = 119;
pub const KEY_PAGE_UP: u16 = 116;
pub const KEY_PAGE_DOWN: u16 = 121;

// ====== Punctuation & Symbols ======
// Note: Shifted versions use the same key code with MOD_SHIFT
// [ ] becomes { } with Shift
// ' becomes " with Shift
// ; becomes : with Shift
// \ becomes | with Shift
// , becomes < with Shift
// / becomes ? with Shift
// . becomes > with Shift

pub const KEY_LEFT_BRACKET: u16 = 33; // [  (Shift = {)
pub const KEY_RIGHT_BRACKET: u16 = 30; // ]  (Shift = })
pub const KEY_QUOTE: u16 = 39; // '  (Shift = ")
pub const KEY_SEMICOLON: u16 = 41; // ;  (Shift = :)
pub const KEY_BACKSLASH: u16 = 42; // \  (Shift = |)
pub const KEY_COMMA: u16 = 43; // ,  (Shift = <)
pub const KEY_SLASH: u16 = 44; // /  (Shift = ?)
pub const KEY_PERIOD: u16 = 47; // .  (Shift = >)
pub const KEY_GRAVE: u16 = 50; // `  (Shift = ~)

// ====== Other Special Keys ======

pub const KEY_HELP: u16 = 114;
pub const KEY_CAPS_LOCK: u16 = 57;
pub const KEY_VOLUME_UP: u16 = 72;
pub const KEY_VOLUME_DOWN: u16 = 73;
pub const KEY_MUTE: u16 = 74;

// ====== Modifier Key Codes ======
// Physical modifier keys (for tracking key up/down events)

pub const KEY_CMD_LEFT: u16 = 55;
pub const KEY_CMD_RIGHT: u16 = 54;
pub const KEY_SHIFT_LEFT: u16 = 56;
pub const KEY_SHIFT_RIGHT: u16 = 60;
pub const KEY_OPTION_LEFT: u16 = 58;
pub const KEY_OPTION_RIGHT: u16 = 61;
pub const KEY_CONTROL_LEFT: u16 = 59;
pub const KEY_CONTROL_RIGHT: u16 = 62;

// ====== Key Name Mapping ======

/// Maps a key name string to its corresponding key code(s).
/// For modifiers, returns both left and right key codes (either side works).
/// For regular keys, returns a single key code.
///
/// Returns None if the key name is not recognized.
pub fn key_name_to_codes(name: &str) -> Option<Vec<u16>> {
    let name_lower = name.to_lowercase();

    let codes = match name_lower.as_str() {
        // Modifiers (return both left and right)
        "cmd" | "command" => vec![KEY_CMD_LEFT, KEY_CMD_RIGHT],
        "shift" => vec![KEY_SHIFT_LEFT, KEY_SHIFT_RIGHT],
        "option" | "alt" => vec![KEY_OPTION_LEFT, KEY_OPTION_RIGHT],
        "control" | "ctrl" => vec![KEY_CONTROL_LEFT, KEY_CONTROL_RIGHT],

        // Letters
        "a" => vec![KEY_A],
        "b" => vec![KEY_B],
        "c" => vec![KEY_C],
        "d" => vec![KEY_D],
        "e" => vec![KEY_E],
        "f" => vec![KEY_F],
        "g" => vec![KEY_G],
        "h" => vec![KEY_H],
        "i" => vec![KEY_I],
        "j" => vec![KEY_J],
        "k" => vec![KEY_K],
        "l" => vec![KEY_L],
        "m" => vec![KEY_M],
        "n" => vec![KEY_N],
        "o" => vec![KEY_O],
        "p" => vec![KEY_P],
        "q" => vec![KEY_Q],
        "r" => vec![KEY_R],
        "s" => vec![KEY_S],
        "t" => vec![KEY_T],
        "u" => vec![KEY_U],
        "v" => vec![KEY_V],
        "w" => vec![KEY_W],
        "x" => vec![KEY_X],
        "y" => vec![KEY_Y],
        "z" => vec![KEY_Z],

        // Numbers (top row)
        "1" => vec![KEY_1],
        "2" => vec![KEY_2],
        "3" => vec![KEY_3],
        "4" => vec![KEY_4],
        "5" => vec![KEY_5],
        "6" => vec![KEY_6],
        "7" => vec![KEY_7],
        "8" => vec![KEY_8],
        "9" => vec![KEY_9],
        "0" => vec![KEY_0],

        // Numbers (keypad)
        "pad_1" => vec![KEYPAD_1],
        "pad_2" => vec![KEYPAD_2],
        "pad_3" => vec![KEYPAD_3],
        "pad_4" => vec![KEYPAD_4],
        "pad_5" => vec![KEYPAD_5],
        "pad_6" => vec![KEYPAD_6],
        "pad_7" => vec![KEYPAD_7],
        "pad_8" => vec![KEYPAD_8],
        "pad_9" => vec![KEYPAD_9],
        "pad_0" => vec![KEYPAD_0],
        "pad_decimal" => vec![KEYPAD_DECIMAL],
        "pad_multiply" => vec![KEYPAD_MULTIPLY],
        "pad_plus" => vec![KEYPAD_PLUS],
        "pad_clear" => vec![KEYPAD_CLEAR],
        "pad_divide" => vec![KEYPAD_DIVIDE],
        "pad_enter" | "pad_return" => vec![KEYPAD_ENTER],
        "pad_minus" => vec![KEYPAD_MINUS],
        "pad_equal" | "pad_equals" => vec![KEYPAD_EQUALS],

        // Function keys
        "f1" => vec![KEY_F1],
        "f2" => vec![KEY_F2],
        "f3" => vec![KEY_F3],
        "f4" => vec![KEY_F4],
        "f5" => vec![KEY_F5],
        "f6" => vec![KEY_F6],
        "f7" => vec![KEY_F7],
        "f8" => vec![KEY_F8],
        "f9" => vec![KEY_F9],
        "f10" => vec![KEY_F10],
        "f11" => vec![KEY_F11],
        "f12" => vec![KEY_F12],
        "f13" => vec![KEY_F13],
        "f14" => vec![KEY_F14],
        "f15" => vec![KEY_F15],
        "f16" => vec![KEY_F16],
        "f17" => vec![KEY_F17],
        "f18" => vec![KEY_F18],
        "f19" => vec![KEY_F19],
        "f20" => vec![KEY_F20],

        // Arrow keys
        "left" => vec![KEY_LEFT],
        "right" => vec![KEY_RIGHT],
        "up" => vec![KEY_UP],
        "down" => vec![KEY_DOWN],

        // Special keys
        "space" => vec![KEY_SPACE],
        "return" | "enter" => vec![KEY_RETURN],
        "tab" => vec![KEY_TAB],
        "delete" | "backspace" => vec![KEY_DELETE],
        "escape" | "esc" => vec![KEY_ESCAPE],
        "home" => vec![KEY_HOME],
        "end" => vec![KEY_END],
        "pageup" | "page_up" => vec![KEY_PAGE_UP],
        "pagedown" | "page_down" => vec![KEY_PAGE_DOWN],

        // Punctuation
        "minus" | "-" | "underscore" | "_" => vec![KEY_MINUS],
        "equal" | "equals" | "=" | "plus" => vec![KEY_EQUAL],
        "leftbracket" | "[" => vec![KEY_LEFT_BRACKET],
        "rightbracket" | "]" => vec![KEY_RIGHT_BRACKET],
        "backslash" | "\\" => vec![KEY_BACKSLASH],
        "semicolon" | ";" => vec![KEY_SEMICOLON],
        "quote" | "'" => vec![KEY_QUOTE],
        "comma" | "," => vec![KEY_COMMA],
        "period" | "." => vec![KEY_PERIOD],
        "slash" | "/" => vec![KEY_SLASH],
        "grave" | "`" => vec![KEY_GRAVE],

        _ => return None,
    };

    Some(codes)
}

/// Maps a key code back to a human-readable name
///
/// For modifier keys, returns generic names (cmd, shift, etc.) without L/R distinction.
pub fn keycode_to_name(code: u16) -> Option<&'static str> {
    match code {
        // Modifiers (map both left and right to same name)
        KEY_CMD_LEFT | KEY_CMD_RIGHT => Some("cmd"),
        KEY_SHIFT_LEFT | KEY_SHIFT_RIGHT => Some("shift"),
        KEY_OPTION_LEFT | KEY_OPTION_RIGHT => Some("option"),
        KEY_CONTROL_LEFT | KEY_CONTROL_RIGHT => Some("ctrl"),

        // Letters
        KEY_A => Some("a"),
        KEY_B => Some("b"),
        KEY_C => Some("c"),
        KEY_D => Some("d"),
        KEY_E => Some("e"),
        KEY_F => Some("f"),
        KEY_G => Some("g"),
        KEY_H => Some("h"),
        KEY_I => Some("i"),
        KEY_J => Some("j"),
        KEY_K => Some("k"),
        KEY_L => Some("l"),
        KEY_M => Some("m"),
        KEY_N => Some("n"),
        KEY_O => Some("o"),
        KEY_P => Some("p"),
        KEY_Q => Some("q"),
        KEY_R => Some("r"),
        KEY_S => Some("s"),
        KEY_T => Some("t"),
        KEY_U => Some("u"),
        KEY_V => Some("v"),
        KEY_W => Some("w"),
        KEY_X => Some("x"),
        KEY_Y => Some("y"),
        KEY_Z => Some("z"),

        // Numbers
        KEY_0 => Some("0"),
        KEY_1 => Some("1"),
        KEY_2 => Some("2"),
        KEY_3 => Some("3"),
        KEY_4 => Some("4"),
        KEY_5 => Some("5"),
        KEY_6 => Some("6"),
        KEY_7 => Some("7"),
        KEY_8 => Some("8"),
        KEY_9 => Some("9"),

        // Function keys
        KEY_F1 => Some("f1"),
        KEY_F2 => Some("f2"),
        KEY_F3 => Some("f3"),
        KEY_F4 => Some("f4"),
        KEY_F5 => Some("f5"),
        KEY_F6 => Some("f6"),
        KEY_F7 => Some("f7"),
        KEY_F8 => Some("f8"),
        KEY_F9 => Some("f9"),
        KEY_F10 => Some("f10"),
        KEY_F11 => Some("f11"),
        KEY_F12 => Some("f12"),
        KEY_F13 => Some("f13"),
        KEY_F14 => Some("f14"),
        KEY_F15 => Some("f15"),
        KEY_F16 => Some("f16"),
        KEY_F17 => Some("f17"),
        KEY_F18 => Some("f18"),
        KEY_F19 => Some("f19"),
        KEY_F20 => Some("f20"),

        // Special keys
        KEY_SPACE => Some("space"),
        KEY_RETURN => Some("return"),
        KEY_TAB => Some("tab"),
        KEY_DELETE => Some("delete"),
        KEY_ESCAPE => Some("esc"),
        KEY_LEFT => Some("left"),
        KEY_RIGHT => Some("right"),
        KEY_UP => Some("up"),
        KEY_DOWN => Some("down"),
        KEY_HOME => Some("home"),
        KEY_END => Some("end"),
        KEY_PAGE_UP => Some("pageup"),
        KEY_PAGE_DOWN => Some("pagedown"),

        // Punctuation
        KEY_MINUS => Some("-"),
        KEY_EQUAL => Some("="),
        KEY_LEFT_BRACKET => Some("["),
        KEY_RIGHT_BRACKET => Some("]"),
        KEY_BACKSLASH => Some("\\"),
        KEY_SEMICOLON => Some(";"),
        KEY_QUOTE => Some("'"),
        KEY_COMMA => Some(","),
        KEY_PERIOD => Some("."),
        KEY_SLASH => Some("/"),
        KEY_GRAVE => Some("`"),

        _ => None,
    }
}
