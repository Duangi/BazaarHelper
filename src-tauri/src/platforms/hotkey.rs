use device_query::{DeviceQuery, DeviceState, MouseState};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

#[cfg(not(target_os = "windows"))]
use device_query::Keycode;

pub fn is_key_pressed(key_code: i32, device_state: &DeviceState, mouse_state: &MouseState) -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe { (GetAsyncKeyState(key_code) as i16) < 0 }
    }

    #[cfg(not(target_os = "windows"))]
    {
        match key_code {
            1 => mouse_state.button_pressed.first().copied().unwrap_or(false),
            2 => mouse_state.button_pressed.get(2).copied().unwrap_or(false),
            4 => mouse_state.button_pressed.get(1).copied().unwrap_or(false),
            18 => {
                device_state.get_keys().contains(&Keycode::LAlt)
                    || device_state.get_keys().contains(&Keycode::RAlt)
            }
            81 => device_state.get_keys().contains(&Keycode::Q),
            192 => device_state.get_keys().contains(&Keycode::Grave),
            65..=90 => {
                let keys = device_state.get_keys();
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
