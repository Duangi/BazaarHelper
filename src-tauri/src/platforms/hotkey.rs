use device_query::{DeviceQuery, DeviceState, MouseState};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

#[cfg(not(target_os = "windows"))]
use device_query::Keycode;

#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

#[cfg(target_os = "macos")]
fn macos_pressed_mouse_buttons_mask() -> Option<u64> {
    unsafe {
        let ns_event_class = class!(NSEvent);
        let mask: u64 = msg_send![ns_event_class, pressedMouseButtons];
        Some(mask)
    }
}

#[cfg(target_os = "macos")]
fn macos_mouse_button_pressed(key_code: i32) -> Option<bool> {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGEventSourceButtonState(state_id: i32, button: i32) -> bool;
    }

    // kCGEventSourceStateCombinedSessionState = 0
    const COMBINED_SESSION_STATE: i32 = 0;
    let button = match key_code {
        1 => 0, // left
        2 => 1, // right
        4 => 2, // center
        _ => return None,
    };

    let quartz = unsafe { CGEventSourceButtonState(COMBINED_SESSION_STATE, button) };
    let appkit = macos_pressed_mouse_buttons_mask()
        .map(|mask| ((mask >> button) & 1) == 1)
        .unwrap_or(false);
    Some(quartz || appkit)
}

pub fn is_key_pressed(key_code: i32, device_state: &DeviceState, mouse_state: &MouseState) -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe { (GetAsyncKeyState(key_code) as i16) < 0 }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let keys = device_state.get_keys();
        let mouse_pressed = |primary_idx: usize, fallback_idx: usize| -> bool {
            mouse_state
                .button_pressed
                .get(primary_idx)
                .copied()
                .unwrap_or(false)
                || mouse_state
                    .button_pressed
                    .get(fallback_idx)
                    .copied()
                    .unwrap_or(false)
        };

        match key_code {
            // Mouse button index differs across platforms/devices.
            // Align with monitor loop's observed mapping first, then fallback.
            1 => {
                #[cfg(target_os = "macos")]
                {
                    if let Some(v) = macos_mouse_button_pressed(1) {
                        return v || mouse_pressed(1, 0);
                    }
                }
                mouse_pressed(1, 0)
            }
            2 => {
                #[cfg(target_os = "macos")]
                {
                    if let Some(v) = macos_mouse_button_pressed(2) {
                        return v || mouse_pressed(3, 2);
                    }
                }
                mouse_pressed(3, 2)
            }
            4 => {
                #[cfg(target_os = "macos")]
                {
                    if let Some(v) = macos_mouse_button_pressed(4) {
                        return v || mouse_pressed(2, 1);
                    }
                }
                mouse_pressed(2, 1)
            }
            18 => {
                keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt)
            }
            81 => keys.contains(&Keycode::Q),
            192 => keys.contains(&Keycode::Grave),
            48 => keys.contains(&Keycode::Key0),
            49 => keys.contains(&Keycode::Key1),
            50 => keys.contains(&Keycode::Key2),
            51 => keys.contains(&Keycode::Key3),
            52 => keys.contains(&Keycode::Key4),
            53 => keys.contains(&Keycode::Key5),
            54 => keys.contains(&Keycode::Key6),
            55 => keys.contains(&Keycode::Key7),
            56 => keys.contains(&Keycode::Key8),
            57 => keys.contains(&Keycode::Key9),
            112 => keys.contains(&Keycode::F1),
            113 => keys.contains(&Keycode::F2),
            114 => keys.contains(&Keycode::F3),
            115 => keys.contains(&Keycode::F4),
            116 => keys.contains(&Keycode::F5),
            117 => keys.contains(&Keycode::F6),
            118 => keys.contains(&Keycode::F7),
            119 => keys.contains(&Keycode::F8),
            120 => keys.contains(&Keycode::F9),
            121 => keys.contains(&Keycode::F10),
            122 => keys.contains(&Keycode::F11),
            123 => keys.contains(&Keycode::F12),
            8 => keys.contains(&Keycode::Backspace),
            9 => keys.contains(&Keycode::Tab),
            13 => keys.contains(&Keycode::Enter),
            16 => keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift),
            17 => keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl),
            20 => keys.contains(&Keycode::CapsLock),
            27 => keys.contains(&Keycode::Escape),
            32 => keys.contains(&Keycode::Space),
            33 => keys.contains(&Keycode::PageUp),
            34 => keys.contains(&Keycode::PageDown),
            35 => keys.contains(&Keycode::End),
            36 => keys.contains(&Keycode::Home),
            37 => keys.contains(&Keycode::Left),
            38 => keys.contains(&Keycode::Up),
            39 => keys.contains(&Keycode::Right),
            40 => keys.contains(&Keycode::Down),
            45 => keys.contains(&Keycode::Insert),
            46 => keys.contains(&Keycode::Delete),
            65..=90 => {
                let keycode = match key_code {
                    65 => Some(Keycode::A),
                    66 => Some(Keycode::B),
                    67 => Some(Keycode::C),
                    68 => Some(Keycode::D),
                    69 => Some(Keycode::E),
                    70 => Some(Keycode::F),
                    71 => Some(Keycode::G),
                    72 => Some(Keycode::H),
                    73 => Some(Keycode::I),
                    74 => Some(Keycode::J),
                    75 => Some(Keycode::K),
                    76 => Some(Keycode::L),
                    77 => Some(Keycode::M),
                    78 => Some(Keycode::N),
                    79 => Some(Keycode::O),
                    80 => Some(Keycode::P),
                    82 => Some(Keycode::R),
                    83 => Some(Keycode::S),
                    84 => Some(Keycode::T),
                    85 => Some(Keycode::U),
                    86 => Some(Keycode::V),
                    87 => Some(Keycode::W),
                    88 => Some(Keycode::X),
                    89 => Some(Keycode::Y),
                    90 => Some(Keycode::Z),
                    _ => None,
                };
                keycode.map(|k| keys.contains(&k)).unwrap_or(false)
            }
            _ => false,
        }
    }
}
