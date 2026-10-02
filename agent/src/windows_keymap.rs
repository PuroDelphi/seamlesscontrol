//! Linux evdev key codes on the wire to Windows Set 1 scan codes.
//! Keep this conversion independent of Win32 so it can be checked on Linux.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanCode {
    pub code: u16,
    pub extended: bool,
}

pub fn evdev_to_set1(key: u32) -> Option<ScanCode> {
    let (code, extended) = match key {
        1..=83 => (key as u16, false),
        86 => (0x56, false), // 102nd key on ISO keyboards
        87 => (0x57, false), // F11
        88 => (0x58, false), // F12
        96 => (0x1c, true),  // keypad Enter
        97 => (0x1d, true),  // right Ctrl
        98 => (0x35, true),  // keypad slash
        99 => (0x37, true),  // Print Screen
        100 => (0x38, true), // right Alt / AltGr
        102 => (0x47, true),
        103 => (0x48, true),
        104 => (0x49, true),
        105 => (0x4b, true),
        106 => (0x4d, true),
        107 => (0x4f, true),
        108 => (0x50, true),
        109 => (0x51, true),
        110 => (0x52, true),
        111 => (0x53, true),
        125 => (0x5b, true), // left Super -> left Windows
        126 => (0x5c, true), // right Super -> right Windows
        127 => (0x5d, true), // context menu
        _ => return None,
    };
    Some(ScanCode { code, extended })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omarchy_super_v_and_navigation_keep_physical_keys() {
        assert_eq!(
            evdev_to_set1(125),
            Some(ScanCode {
                code: 0x5b,
                extended: true
            })
        );
        assert_eq!(
            evdev_to_set1(47),
            Some(ScanCode {
                code: 0x2f,
                extended: false
            })
        );
        assert_eq!(
            evdev_to_set1(105),
            Some(ScanCode {
                code: 0x4b,
                extended: true
            })
        );
        assert_eq!(
            evdev_to_set1(97),
            Some(ScanCode {
                code: 0x1d,
                extended: true
            })
        );
        assert_eq!(evdev_to_set1(119), None); // Pause needs a special sequence.
    }
}
