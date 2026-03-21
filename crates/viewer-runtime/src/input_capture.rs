use xconnect_protocol::InputEvent;

pub struct InputCapture;

impl InputCapture {
    pub fn normalize_mouse(x: f32, y: f32) -> InputEvent {
        InputEvent::MouseMove {
            x,
            y,
            normalized: true,
        }
    }

    pub fn from_absolute(
        abs_x: i32,
        abs_y: i32,
        viewport_width: i32,
        viewport_height: i32,
    ) -> InputEvent {
        let safe_w = viewport_width.max(1) as f32;
        let safe_h = viewport_height.max(1) as f32;

        let norm_x = (abs_x as f32 / safe_w).clamp(0.0, 1.0);
        let norm_y = (abs_y as f32 / safe_h).clamp(0.0, 1.0);

        Self::normalize_mouse(norm_x, norm_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_input_is_normalized() {
        let event = InputCapture::from_absolute(960, 540, 1920, 1080);
        match event {
            InputEvent::MouseMove { x, y, normalized } => {
                assert!(normalized);
                assert!((x - 0.5).abs() < 0.001);
                assert!((y - 0.5).abs() < 0.001);
            }
            _ => panic!("unexpected event"),
        }
    }

    #[test]
    fn normalization_clamps_bounds() {
        let event = InputCapture::from_absolute(-10, 2000, 1920, 1080);
        match event {
            InputEvent::MouseMove { x, y, .. } => {
                assert_eq!(x, 0.0);
                assert_eq!(y, 1.0);
            }
            _ => panic!("unexpected event"),
        }
    }
}
