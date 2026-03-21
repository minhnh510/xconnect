use std::sync::{Mutex, OnceLock};

use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use xconnect_protocol::InputEvent;

use crate::HostRuntimeError;

fn enigo_instance() -> Result<&'static Mutex<Enigo>, HostRuntimeError> {
    static INSTANCE: OnceLock<Result<Mutex<Enigo>, String>> = OnceLock::new();
    let result = INSTANCE.get_or_init(|| {
        Enigo::new(&Settings::default())
            .map(Mutex::new)
            .map_err(|err| format!("enigo init failed: {err}"))
    });

    match result {
        Ok(m) => Ok(m),
        Err(err) => Err(HostRuntimeError::Runtime(err.clone())),
    }
}

pub fn apply_input_event(event: &InputEvent) -> Result<(), HostRuntimeError> {
    let enigo = enigo_instance()?;
    let mut guard = enigo
        .lock()
        .map_err(|_| HostRuntimeError::Runtime("enigo lock poisoned".to_string()))?;

    match event {
        InputEvent::MouseMove {
            x,
            y,
            normalized: _,
        } => {
            guard
                .move_mouse(*x as i32, *y as i32, Coordinate::Abs)
                .map_err(|err| HostRuntimeError::Runtime(format!("move mouse failed: {err}")))?;
        }
        InputEvent::MouseButton { button, pressed } => {
            let btn = match button {
                1 => Button::Left,
                2 => Button::Middle,
                3 => Button::Right,
                _ => return Ok(()),
            };
            let direction = if *pressed {
                Direction::Press
            } else {
                Direction::Release
            };
            guard
                .button(btn, direction)
                .map_err(|err| HostRuntimeError::Runtime(format!("mouse button failed: {err}")))?;
        }
        InputEvent::Wheel { delta_x, delta_y } => {
            if *delta_x != 0 {
                guard
                    .scroll(*delta_x, enigo::Axis::Horizontal)
                    .map_err(|err| {
                        HostRuntimeError::Runtime(format!("horizontal scroll failed: {err}"))
                    })?;
            }
            if *delta_y != 0 {
                guard
                    .scroll(*delta_y, enigo::Axis::Vertical)
                    .map_err(|err| {
                        HostRuntimeError::Runtime(format!("vertical scroll failed: {err}"))
                    })?;
            }
        }
        InputEvent::Key {
            key_code,
            pressed,
            modifiers: _,
        } => {
            if let Some(ch) = char::from_u32(*key_code) {
                let direction = if *pressed {
                    Direction::Press
                } else {
                    Direction::Release
                };
                guard
                    .key(Key::Unicode(ch), direction)
                    .map_err(|err| HostRuntimeError::Runtime(format!("key event failed: {err}")))?;
            }
        }
    }

    Ok(())
}
