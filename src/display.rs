pub struct Display {
    display: [bool; 2048], // Display Buffer
}

impl Display {
    pub fn new() -> Display {
        return Display {
            display: [false; 2048],
        };
    }
    pub fn _clear(&mut self) {
        self.display = [false; 2048];
    }
    pub fn pixels(&self) -> &[bool; 2048] {
        &self.display
    }
    fn pixel(&mut self, pixel: usize, sprite: bool) -> bool {
        let result: bool = self.display[pixel] && sprite;
        self.display[pixel] ^= sprite;
        return result;
    }
    pub fn draw(&mut self, x: u8, y: u8, rows: &[u8]) -> bool {
        let mut collision = false;
        for (i, row) in rows.iter().enumerate() {
            for j in 0..8 {
                if (x as usize + j) < 64 && (y as usize + i) < 32 {
                    collision |= self.pixel(
                        (y as usize + i as usize) * 64 + x as usize + j as usize,
                        ((*row >> 7 - j) & 1) != 0,
                    );
                }
            }
        }
        return collision;
    }
}
