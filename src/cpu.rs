use crate::display::Display;
use crate::keys::Keyboard;
use crate::time::Timer;

const FONT: &[u8] = &[
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Cpu {
    v: [u8; 16],        // Registers V0 -> VF
    i: u16,             // Index Register I
    pc: u16,            // Program Counter
    memory: [u8; 4096], // Memory
    stack: [u16; 16],   //Call Stack
    sp: u8,             //Stack Pointer
    timer: Timer,       // Timer
    display: Display,
    keys: Keyboard,
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut memory = [0x00; 4096];
        memory[0x0050..0x00A0].copy_from_slice(FONT);
        return Cpu {
            v: [0x00; 16],
            i: 0x0000,
            pc: 0x0200,
            memory,
            stack: [0x0000; 16],
            sp: 0x00,
            timer: Timer::new(),
            display: Display::new(),
            keys: Keyboard::new(),
        };
    }

    pub fn decode(&mut self) -> Result<(), String> {
        let ins: u16 = self.fetch()?;

        match ins >> 12 {
            // MISC OPERATIONS
            0x0 => match ins {
                //CLEAR SCREEN
                0x00E0 => self.display._clear(),
                //RETURN FROM SUBROUTINE
                0x00EE => {
                    if self.sp != 0 {
                        self.pc = self.stack[self.sp as usize - 0x1];
                        self.sp -= 0x1;
                    } else {
                        return Err(format!("Stack underflow"));
                    }
                }
                _ => {
                    return Err(format!(
                        "Invalid Instruction at {:04X}",
                        self.pc.wrapping_sub(2)
                    ));
                }
            },
            //JUMP
            0x1 => self.pc = ins & 0xFFF,
            //CALL SUBRUTINE
            0x2 => {
                if self.sp < 16 {
                    self.stack[self.sp as usize] = self.pc;
                    self.pc = ins & 0xFFF;
                    self.sp += 1;
                } else {
                    return Err(format!("Stack overflow"));
                }
            }
            // SKIP IF EQUAL
            0x3 => {
                if self.v[((ins & 0x0F00) >> 8) as usize] as u16 == ins & 0x00FF {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // SKIP IF NOT EQUAL
            0x4 => {
                if self.v[((ins & 0x0F00) >> 8) as usize] as u16 != ins & 0x00FF {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // COMPARE REGISTERS
            0x5 => {
                if ins & 0x000F != 0x0 {
                    return Err(format!(
                        "Invalid Instruction at {:04X}",
                        self.pc.wrapping_sub(2)
                    ));
                }
                if self.v[((ins & 0x0F00) >> 8) as usize] as u16
                    == self.v[((ins & 0x00F0) >> 4) as usize] as u16
                {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            // LOAD IMMEDIATE
            0x6 => self.v[((ins & 0x0F00) >> 8) as usize] = (ins & 0x00FF) as u8,
            // ADD IMMEDIATE
            0x7 => {
                let x = ((ins & 0x0F00) >> 8) as usize;
                self.v[x] = self.v[x].wrapping_add((ins & 0x00FF) as u8);
            }
            // REGISTER OPERATIONS
            0x8 => {
                let x = ((ins & 0x0F00) >> 8) as usize;
                let y = ((ins & 0x00F0) >> 4) as usize;

                match ins & 0x000F {
                    // COPY FROM Y into X
                    0x0 => self.v[x] = self.v[y],
                    // BITWISE OR AND SAVE INTO X
                    0x1 => self.v[x] |= self.v[y],
                    // BITWISE AND AND SAVE INTO X
                    0x2 => self.v[x] &= self.v[y],
                    // BITWISE XOR AND SAVE INTO X
                    0x3 => self.v[x] ^= self.v[y],
                    //ADD REGISTERS
                    0x4 => {
                        let overflow: bool;
                        (self.v[x], overflow) = self.v[x].overflowing_add(self.v[y]);
                        self.v[0xF] = overflow as u8;
                    }
                    //SUBTRACT REGISTERS
                    0x5 => {
                        let overflow: bool;
                        (self.v[x], overflow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[0xF] = !overflow as u8;
                    }
                    // RIGHT SHIFT
                    0x6 => {
                        let vx = self.v[x];
                        self.v[x] >>= 0x1;
                        self.v[0xF] = vx & 0x1;
                    }
                    // SUBTRACT NUMBER
                    0x7 => {
                        let overflow: bool;
                        (self.v[x], overflow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[0xF] = !overflow as u8;
                    }
                    // LEFT SHIFT
                    0xE => {
                        let vx = self.v[x];
                        self.v[x] <<= 0x1;
                        self.v[0xF] = (vx & 0x80) >> 7;
                    }
                    _ => {
                        return Err(format!(
                            "Invalid Instruction at {:04X}",
                            self.pc.wrapping_sub(2)
                        ));
                    }
                }
            }
            // SKIP IF X NOT EQUAL TO Y
            0x9 => {
                if ins & 0x000F != 0x0 {
                    return Err(format!(
                        "Invalid Instruction at {:04X}",
                        self.pc.wrapping_sub(2)
                    ));
                }
                if self.v[((ins & 0x0F00) >> 8) as usize] as u16
                    != self.v[((ins & 0x00F0) >> 4) as usize] as u16
                {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            0xA => self.i = ins & 0xFFF,
            0xB => self.pc = (ins & 0xFFF) + self.v[0] as u16,
            0xC => {
                self.v[((ins & 0x0F00) >> 8) as usize] = rand::random::<u8>() & (ins & 0x00FF) as u8
            }
            0xD => {
                if self.i as usize + (ins & 0x000F) as usize > 4096 {
                    return Err(format!("Out Of Mem"));
                };
                self.v[0xF] = self.display.draw(
                    self.v[((ins & 0x0F00) >> 8) as usize],
                    self.v[((ins & 0x00F0) >> 4) as usize],
                    &self.memory
                        [(self.i as usize)..((self.i as usize + (ins & 0x000F) as usize) as usize)],
                ) as u8;
            }
            0xE => match ins & 0x00FF {
                0x9E => {
                    if self.keys.get(self.v[((ins & 0x0F00) >> 8) as usize])? {
                        self.pc = self.pc.wrapping_add(2);
                    }
                }
                0xA1 => {
                    if !self.keys.get(self.v[((ins & 0x0F00) >> 8) as usize])? {
                        self.pc = self.pc.wrapping_add(2)
                    }
                }
                _ => {
                    return Err(format!(
                        "Invalid Instruction at {:04X}",
                        self.pc.wrapping_sub(2)
                    ));
                }
            },
            0xF => match (ins & 0x00FF) as u8 {
                0x07 => self.v[((ins & 0x0F00) >> 8) as usize] = self.timer.dlt,
                0x0A => {
                    let key: Option<u8> = self.keys.get_pressed();
                    if let Some(key) = key {
                        self.v[((ins & 0x0F00) >> 8) as usize] = key;
                    } else {
                        self.pc = self.pc.wrapping_sub(2);
                    }
                }
                0x15 => self.timer.dlt = self.v[((ins & 0x0F00) >> 8) as usize],
                0x18 => self.timer.sdt = self.v[((ins & 0x0F00) >> 8) as usize],
                0x1E => {
                    self.i = self
                        .i
                        .wrapping_add(self.v[((ins & 0x0F00) >> 8) as usize] as u16)
                }
                0x29 => {
                    let x: u8 = self.v[((ins & 0x0F00) >> 8) as usize];
                    if x > 0x0F {
                        return Err(format!("Pointer Error when loading Font"));
                    }
                    self.i = 0x0050 + (x as u16 * 5);
                }
                0x33 => {
                    let x: u8 = self.v[((ins & 0x0F00) >> 8) as usize];
                    if self.i as usize + 3 > 4096 {
                        return Err(format!("BCD Operation Will result in memory overflow"));
                    }
                    self.memory[self.i as usize] = x / 100;
                    self.memory[self.i as usize + 1] = (x / 10) % 10;
                    self.memory[self.i as usize + 2] = x % 10;
                }
                0x55 => {
                    let x = ((ins & 0x0F00) >> 8) as usize;
                    if self.i as usize + x + 1 > 4096 {
                        return Err(format!("Register Copy results in a Memory overflow"));
                    }
                    for i in 0..=x {
                        self.memory[self.i as usize + i] = self.v[i]
                    }
                }
                0x65 => {
                    let x = ((ins & 0x0F00) >> 8) as usize;
                    if self.i as usize + x + 1 > 4096 {
                        return Err(format!("Register Copy results in a Out Of Index Operation"));
                    }
                    for i in 0..=x {
                        self.v[i] = self.memory[self.i as usize + i]
                    }
                }
                _ => {
                    return Err(format!(
                        "Invalid Instruction at {:04X}",
                        self.pc.wrapping_sub(2)
                    ));
                }
            },
            _ => {
                return Err(format!(
                    "Invalid Instruction at {:04X}",
                    self.pc.wrapping_sub(2)
                ));
            }
        }

        Ok(())
    }

    pub fn fetch(&mut self) -> Result<u16, String> {
        if self.pc as usize + 1 >= 4096 {
            return Err(format!("PC leak"));
        }
        let op: u16 = ((self.memory[self.pc as usize] as u16) << 8)
            | self.memory[(self.pc + 0x1) as usize] as u16;
        self.pc = self.pc.wrapping_add(2);
        return Ok(op);
    }
}

impl Cpu {
    pub fn _parse(&mut self, program: &[u8]) -> Result<(), String> {
        if program.len() > 3584 {
            return Err(String::from("Program is too large"));
        }
        let mut i: usize = 0x0200;
        for inst in program {
            self.memory[i] = *inst;
            i += 1;
        }
        self.pc = 0x0200;
        return Ok(());
    }
}

pub struct CpuView<'a> {
    pub v: &'a [u8; 16],
    pub memory: &'a [u8; 4096],
    pub display: &'a [bool; 2048],
    pub stack: &'a [u16; 16],
    pub i: u16,
    pub pc: u16,
    pub sp: u8,
    pub dlt: u8,
    pub sdt: u8,
}

impl Cpu {
    pub fn ui_view(&self) -> CpuView<'_> {
        CpuView {
            v: &self.v,
            memory: &self.memory,
            display: self.display.pixels(),
            stack: &self.stack,
            i: self.i,
            pc: self.pc,
            sp: self.sp,
            dlt: self.timer.dlt,
            sdt: self.timer.sdt,
        }
    }

    pub fn ui_key(&mut self, key: u8, pressed: bool) {
        self.keys.set(key, pressed);
    }

    pub fn ui_tick_timers(&mut self) {
        self.timer.tick();
    }
}
