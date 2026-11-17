pub struct Keyboard {
    keys: [bool; 16], // Display Buffer
}

impl Keyboard {
    pub fn new() -> Keyboard {
        return Keyboard { keys: [false; 16] };
    }
    pub fn get(&self, key: u8) -> Result<bool, String> {
        if key > 0xF {
            return Err(format!(
                "Invalid keyboard Acces. Register Value is too large"
            ));
        }
        return Ok(self.keys[key as usize]);
    }
    pub fn get_pressed(&self) -> Option<u8> {
        for i in 0..16 {
            if self.keys[i] {
                return Some(i as u8);
            }
        }
        return None;
    }
    pub fn set(&mut self, key: u8, pressed: bool) {
        if let Some(slot) = self.keys.get_mut(key as usize) {
            *slot = pressed;
        }
    }
}
