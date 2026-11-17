mod cpu;
mod display;
mod keys;
mod time;

use cpu::Cpu;

#[derive(Default)]
struct Esperat<'a> {
    v: &'a [(usize, u8)],
    i: Option<u16>,
    pc: Option<u16>,
    mem: &'a [(usize, u8)],
    timers: Option<(u8, u8)>,
}

fn comprova(nom: &str, rom: &[u8], passos: usize, esperat: Esperat<'_>) -> Result<(), String> {
    let mut cpu = Cpu::new();
    cpu._parse(rom)?;

    for pas in 0..passos {
        cpu.decode()
            .map_err(|e| format!("{nom}, instrucció {}: {e}", pas + 1))?;
    }

    let (v, i, pc, mem, dlt, sdt) = cpu._snapshot();

    for &(index, valor) in esperat.v {
        assert_eq!(v[index], valor, "{nom}: V{index:X} incorrecte");
    }

    if let Some(valor) = esperat.i {
        assert_eq!(i, valor, "{nom}: I incorrecte");
    }

    if let Some(valor) = esperat.pc {
        assert_eq!(pc, valor, "{nom}: PC incorrecte");
    }

    for &(adreca, valor) in esperat.mem {
        assert_eq!(mem[adreca], valor, "{nom}: memòria {adreca:04X}");
    }

    if let Some((dt, st)) = esperat.timers {
        assert_eq!(dlt, dt, "{nom}: Delay Timer incorrecte");
        assert_eq!(sdt, st, "{nom}: Sound Timer incorrecte");
    }

    println!("[OK] {nom}");
    Ok(())
}

fn comprova_error(nom: &str, rom: &[u8], passos_previs: usize) -> Result<(), String> {
    let mut cpu = Cpu::new();
    cpu._parse(rom)?;

    for _ in 0..passos_previs {
        cpu.decode()?;
    }

    assert!(cpu.decode().is_err(), "{nom}: s'esperava un error");

    println!("[OK] {nom} (error detectat)");
    Ok(())
}

fn main() -> Result<(), String> {
    println!("=== CHIP-8: PROVES DE LA CPU ===\n");

    // 1NNN - JP
    comprova(
        "1NNN - JP",
        &[0x12, 0x06, 0x60, 0x01, 0x60, 0x02, 0x60, 0x07],
        2,
        Esperat {
            v: &[(0, 0x07)],
            pc: Some(0x0208),
            ..Default::default()
        },
    )?;

    // 2NNN - CALL / 00EE - RET
    comprova(
        "2NNN + 00EE - CALL/RET",
        &[
            0x22, 0x06, // CALL 206
            0x60, 0x0B, // V0 = 11
            0x12, 0x0A, // JP 20A
            0x61, 0x07, // V1 = 7
            0x00, 0xEE, // RET
            0x60, 0x09, // V0 = 9
        ],
        6,
        Esperat {
            v: &[(0, 9), (1, 7)],
            pc: Some(0x020C),
            ..Default::default()
        },
    )?;

    // 3XNN, 4XNN, 5XY0, 9XY0
    comprova(
        "3XNN - SE immediate",
        &[0x60, 0x05, 0x30, 0x05, 0x61, 0xEE, 0x61, 0x07],
        3,
        Esperat {
            v: &[(1, 7)],
            pc: Some(0x0208),
            ..Default::default()
        },
    )?;

    comprova(
        "4XNN - SNE immediate",
        &[0x60, 0x05, 0x40, 0x06, 0x61, 0xEE, 0x61, 0x09],
        3,
        Esperat {
            v: &[(1, 9)],
            pc: Some(0x0208),
            ..Default::default()
        },
    )?;

    comprova(
        "5XY0 - SE registers",
        &[0x60, 0x07, 0x61, 0x07, 0x50, 0x10, 0x62, 0xEE, 0x62, 0x09],
        4,
        Esperat {
            v: &[(2, 9)],
            ..Default::default()
        },
    )?;

    comprova(
        "9XY0 - SNE registers",
        &[0x60, 0x07, 0x61, 0x06, 0x90, 0x10, 0x62, 0xEE, 0x62, 0x09],
        4,
        Esperat {
            v: &[(2, 9)],
            ..Default::default()
        },
    )?;

    // 6XNN i 7XNN
    comprova(
        "6XNN + 7XNN - LD/ADD wrapping",
        &[0x60, 0xFE, 0x70, 0x05],
        2,
        Esperat {
            v: &[(0, 0x03)],
            ..Default::default()
        },
    )?;

    // Grup 8XYN
    comprova(
        "8XY0 - LD",
        &[0x60, 0x55, 0x61, 0xAA, 0x80, 0x10],
        3,
        Esperat {
            v: &[(0, 0xAA)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY1 - OR",
        &[0x60, 0x50, 0x61, 0x0F, 0x80, 0x11],
        3,
        Esperat {
            v: &[(0, 0x5F)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY2 - AND",
        &[0x60, 0x5F, 0x61, 0x0F, 0x80, 0x12],
        3,
        Esperat {
            v: &[(0, 0x0F)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY3 - XOR",
        &[0x60, 0x5A, 0x61, 0x0F, 0x80, 0x13],
        3,
        Esperat {
            v: &[(0, 0x55)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY4 - ADD amb carry",
        &[0x60, 0xFF, 0x61, 0x01, 0x80, 0x14],
        3,
        Esperat {
            v: &[(0, 0x00), (15, 1)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY5 - SUB",
        &[0x60, 0x05, 0x61, 0x02, 0x80, 0x15],
        3,
        Esperat {
            v: &[(0, 3), (15, 1)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY6 - SHR",
        &[0x60, 0x03, 0x80, 0x06],
        2,
        Esperat {
            v: &[(0, 1), (15, 1)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XY7 - SUBN",
        &[0x60, 0x02, 0x61, 0x05, 0x80, 0x17],
        3,
        Esperat {
            v: &[(0, 3), (15, 1)],
            ..Default::default()
        },
    )?;

    comprova(
        "8XYE - SHL",
        &[0x60, 0x81, 0x80, 0x0E],
        2,
        Esperat {
            v: &[(0, 2), (15, 1)],
            ..Default::default()
        },
    )?;

    // ANNN - LD I / BNNN - JP V0
    comprova(
        "ANNN - LD I",
        &[0xA3, 0x00],
        1,
        Esperat {
            i: Some(0x0300),
            ..Default::default()
        },
    )?;

    comprova(
        "BNNN - JP V0",
        &[
            0x60, 0x04, 0xB2, 0x06, 0x60, 0xEE, 0x60, 0xEE, 0x60, 0xEE, 0x61, 0xAB,
        ],
        3,
        Esperat {
            v: &[(1, 0xAB)],
            pc: Some(0x020C),
            ..Default::default()
        },
    )?;

    // CXNN - RND (màscara zero = resultat zero)
    comprova(
        "CXNN - RND",
        &[0xC0, 0x00],
        1,
        Esperat {
            v: &[(0, 0)],
            ..Default::default()
        },
    )?;

    // FX29 + DXYN - Dibuixar dues vegades causa col·lisió
    comprova(
        "FX29 + DXYN - Collision",
        &[
            0x60, 0x0A, // V0 = A
            0x61, 0x0A, // X = 10
            0x62, 0x05, // Y = 5
            0xF0, 0x29, // I = sprite A
            0xD1, 0x25, // Draw
            0xD1, 0x25, // Draw una altra vegada
        ],
        6,
        Esperat {
            v: &[(15, 1)],
            i: Some(0x0082),
            ..Default::default()
        },
    )?;

    // 00E0 - CLS ha d'eliminar la col·lisió
    comprova(
        "00E0 - CLS",
        &[
            0x60, 0x0A, 0x61, 0x0A, 0x62, 0x05, 0xF0, 0x29, 0xD1, 0x25, 0x00, 0xE0, 0xD1, 0x25,
        ],
        7,
        Esperat {
            v: &[(15, 0)],
            ..Default::default()
        },
    )?;

    // EX9E i EXA1 amb totes les tecles sense prémer
    comprova(
        "EX9E - SKP (sense tecla)",
        &[0x60, 0x03, 0xE0, 0x9E, 0x61, 0x05],
        3,
        Esperat {
            v: &[(1, 5)],
            pc: Some(0x0206),
            ..Default::default()
        },
    )?;

    comprova(
        "EXA1 - SKNP (sense tecla)",
        &[0x60, 0x03, 0xE0, 0xA1, 0x61, 0xEE, 0x61, 0x07],
        3,
        Esperat {
            v: &[(1, 7)],
            pc: Some(0x0208),
            ..Default::default()
        },
    )?;

    // FX07, FX15, FX18, FX1E
    comprova(
        "FX07/15/18/1E - Timers i I",
        &[
            0x60, 0x0A, // V0 = 10
            0xF0, 0x15, // DLT = V0
            0xF0, 0x18, // SDT = V0
            0xF1, 0x07, // V1 = DLT
            0xA3, 0x00, // I = 0x300
            0xF0, 0x1E, // I += V0
        ],
        6,
        Esperat {
            v: &[(0, 10), (1, 10)],
            i: Some(0x030A),
            timers: Some((10, 10)),
            ..Default::default()
        },
    )?;

    // FX0A sense cap tecla: repeteix la instrucció
    comprova(
        "FX0A - WAIT (sense tecla)",
        &[0xF2, 0x0A, 0x63, 0x11],
        2,
        Esperat {
            v: &[(2, 0), (3, 0)],
            pc: Some(0x0200),
            ..Default::default()
        },
    )?;

    // FX29 - Comprovar adreça i contingut de la font A
    comprova(
        "FX29 - FONT",
        &[0x60, 0x0A, 0xF0, 0x29],
        2,
        Esperat {
            i: Some(0x0082),
            mem: &[
                (0x0082, 0xF0),
                (0x0083, 0x90),
                (0x0084, 0xF0),
                (0x0085, 0x90),
                (0x0086, 0x90),
            ],
            ..Default::default()
        },
    )?;

    // FX33 - BCD
    comprova(
        "FX33 - BCD",
        &[0x63, 0xEA, 0xA3, 0x00, 0xF3, 0x33],
        3,
        Esperat {
            v: &[(3, 234)],
            i: Some(0x0300),
            mem: &[(0x0300, 2), (0x0301, 3), (0x0302, 4)],
            ..Default::default()
        },
    )?;

    // FX55 i FX65 - Guardar i recuperar registres
    comprova(
        "FX55 + FX65 - Store/Load",
        &[
            0x60, 0x12, 0x61, 0x34, 0xA3, 0x00, 0xF1, 0x55, 0x60, 0x00, 0x61, 0x00, 0xF1, 0x65,
        ],
        7,
        Esperat {
            v: &[(0, 0x12), (1, 0x34)],
            i: Some(0x0300),
            mem: &[(0x0300, 0x12), (0x0301, 0x34)],
            pc: Some(0x020E),
            ..Default::default()
        },
    )?;

    println!("\n=== PROVES D'ERROR ===\n");

    comprova_error("Opcode invalid", &[0x00, 0x00], 0)?;
    comprova_error("RET - Stack underflow", &[0x00, 0xEE], 0)?;
    comprova_error("CALL - Stack overflow", &[0x22, 0x00], 16)?;
    comprova_error("5XY1 - Opcode invalid", &[0x50, 0x11], 0)?;
    comprova_error("9XY1 - Opcode invalid", &[0x90, 0x11], 0)?;

    comprova_error("EX9E - Tecla invalida", &[0x60, 0x10, 0xE0, 0x9E], 1)?;

    comprova_error("FX29 - Caracter invalid", &[0x60, 0x10, 0xF0, 0x29], 1)?;

    comprova_error(
        "DXYN - Memoria fora de limits",
        &[0xAF, 0xFF, 0xD0, 0x12],
        1,
    )?;

    comprova_error(
        "FX33 - Memoria fora de limits",
        &[0xAF, 0xFF, 0xF0, 0x33],
        1,
    )?;

    comprova_error(
        "FX55 - Memoria fora de limits",
        &[0xAF, 0xFF, 0xF1, 0x55],
        1,
    )?;

    comprova_error(
        "FX65 - Memoria fora de limits",
        &[0xAF, 0xFF, 0xF1, 0x65],
        1,
    )?;

    comprova_error("FETCH - PC fora de memoria", &[0x1F, 0xFF], 1)?;

    println!("\nTotes les proves executades han passat!");
    Ok(())
}
