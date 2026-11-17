pub struct Timer {
    pub dlt: u8, //Delay Timer
    pub sdt: u8, // Sound Timer
}

impl Timer {
    pub fn new() -> Timer {
        return Timer { dlt: 0x0, sdt: 0x0 };
    }
    pub fn tick(&mut self) {
        self.dlt = self.dlt.saturating_sub(1);
        self.sdt = self.sdt.saturating_sub(1);
    }
}
