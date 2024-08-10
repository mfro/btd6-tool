use std::mem::size_of;

use windows::Win32::{
    System::Diagnostics::Debug::Beep,
    UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
        VIRTUAL_KEY,
    },
};

pub fn beep() {
    unsafe {
        Beep(500, 200).unwrap();
    }
}

pub fn send_input(input: &[INPUT]) -> u32 {
    unsafe { SendInput(&input, size_of::<INPUT>() as _) }
}

pub fn make_input_scancode(scan_code: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: windows::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT {
                wVk: VIRTUAL_KEY::default(),
                wScan: scan_code,
                dwFlags: KEYEVENTF_SCANCODE | flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

pub fn make_keypress_scancode(scan_code: u16) -> [INPUT; 2] {
    [
        make_input_scancode(scan_code, KEYBD_EVENT_FLAGS::default()),
        make_input_scancode(scan_code, KEYEVENTF_KEYUP),
    ]
}
