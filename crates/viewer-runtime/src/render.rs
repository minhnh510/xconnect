use crate::ViewerRuntimeError;

#[derive(Default)]
pub struct Renderer {
    last_frame_bytes: usize,
}

impl Renderer {
    pub fn present_h264(&mut self, encoded_frame: &[u8]) -> Result<(), ViewerRuntimeError> {
        if encoded_frame.is_empty() {
            return Err(ViewerRuntimeError::Runtime("empty frame".to_string()));
        }
        self.last_frame_bytes = encoded_frame.len();
        Ok(())
    }

    pub fn last_frame_size(&self) -> usize {
        self.last_frame_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_rejects_empty_frame() {
        let mut renderer = Renderer::default();
        assert!(renderer.present_h264(&[]).is_err());
    }

    #[test]
    fn renderer_tracks_frame_size() {
        let mut renderer = Renderer::default();
        renderer.present_h264(&[1, 2, 3]).expect("present");
        assert_eq!(renderer.last_frame_size(), 3);
    }
}
