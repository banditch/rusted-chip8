# Rusted CHIP-8

A CHIP-8 emulator written in Rust, featuring a retro-style debugger.

**Final version: v0.3.0**

## Features

- CHIP-8 CPU and 64 × 32 display
- ROM loading and execution
- Step-by-step debugging, breakpoints and register inspection
- Hexadecimal and ASCII memory viewer
- Configurable keyboard mappings
- Separate video output window

## Run

Requires Rust and Cargo.

```bash
cargo run --release
```

Open a ROM using **⌘O**. The emulator starts paused.

## Shortcuts (macOS)

| Shortcut | Action   |
| -------- | -------- |
| ⌘O       | Open ROM |
| ⌘1       | Step     |
| ⌘2       | Run      |
| ⌘3       | Pause    |

CHIP-8 game keys can be reassigned directly in the debugger.

## ROMs

You can find games to try in the [CHIP-8 ROM collection](https://github.com/kripod/chip8-roms/tree/master/games).

Some ROMs may require CHIP-8 compatibility quirks not supported by this emulator.
