//! CHIP-8 — depurador retro-modern (egui / eframe 0.37.x).
//! Manté la lògica de CPU, Display, Keyboard i Timer als teus mòduls.
//! main.rs: ui::run()
//! Necessita la mateixa API d'integració de la UI anterior: CpuView,
//! Cpu::ui_view(), Cpu::ui_key() i Cpu::ui_tick_timers().

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cpu::{Cpu, CpuView};
use eframe::egui::{self, Color32, RichText, Stroke};

// Colors d'eina d'escriptori: gris de sistema, tinta i accents sobris.
const BACKGROUND: Color32 = Color32::from_rgb(247, 248, 248);
const PANEL: Color32 = Color32::from_rgb(250, 251, 251);
const HEADER: Color32 = Color32::from_rgb(236, 238, 239);
const LINE: Color32 = Color32::from_rgb(208, 212, 215);
const INK: Color32 = Color32::from_rgb(43, 47, 51);
const DIM: Color32 = Color32::from_rgb(111, 116, 122);
const BLUE: Color32 = Color32::from_rgb(47, 91, 130);
const GREEN: Color32 = Color32::from_rgb(61, 110, 86);
const AMBER: Color32 = Color32::from_rgb(133, 103, 65);
const PAUSE_RED: Color32 = Color32::from_rgb(185, 48, 51);
const KEY_ACTIVE: Color32 = Color32::from_rgb(12, 135, 73);
const KEY_ACTIVE_FILL: Color32 = Color32::from_rgb(225, 246, 233);
const CURRENT: Color32 = Color32::from_rgb(224, 234, 243);
const VIDEO_BACKGROUND: Color32 = Color32::from_rgb(11, 17, 15);
const VIDEO_COLOR: Color32 = Color32::from_rgb(111, 186, 130);

const DEMO: &[u8] = &[
    0x00, 0xE0, 0x60, 0x0A, 0xF0, 0x29, 0x61, 0x0A, 0x62, 0x08, 0xD1, 0x25, 0x60, 0x0B, 0xF0, 0x29,
    0x61, 0x16, 0xD1, 0x25, 0x60, 0x0C, 0xF0, 0x29, 0x61, 0x22, 0xD1, 0x25, 0x12, 0x1C,
];

// Mapa per defecte: índex del registre de tecla CHIP-8 -> tecla física.
// Es pot canviar directament sota el keypad, sense recompilar.
const DEFAULT_KEYMAP: [egui::Key; 16] = [
    egui::Key::X,    // 0
    egui::Key::Num1, // 1
    egui::Key::Num2, // 2
    egui::Key::Num3, // 3
    egui::Key::Q,    // 4
    egui::Key::W,    // 5
    egui::Key::E,    // 6
    egui::Key::A,    // 7
    egui::Key::S,    // 8
    egui::Key::D,    // 9
    egui::Key::Z,    // A
    egui::Key::C,    // B
    egui::Key::Num4, // C
    egui::Key::R,    // D
    egui::Key::F,    // E
    egui::Key::V,    // F
];

// Només tecles alfanumèriques i les quatre fletxes.
const ALLOWED_KEYS: &[egui::Key] = &[
    egui::Key::Num0,
    egui::Key::Num1,
    egui::Key::Num2,
    egui::Key::Num3,
    egui::Key::Num4,
    egui::Key::Num5,
    egui::Key::Num6,
    egui::Key::Num7,
    egui::Key::Num8,
    egui::Key::Num9,
    egui::Key::A,
    egui::Key::B,
    egui::Key::C,
    egui::Key::D,
    egui::Key::E,
    egui::Key::F,
    egui::Key::G,
    egui::Key::H,
    egui::Key::I,
    egui::Key::J,
    egui::Key::K,
    egui::Key::L,
    egui::Key::M,
    egui::Key::N,
    egui::Key::O,
    egui::Key::P,
    egui::Key::Q,
    egui::Key::R,
    egui::Key::S,
    egui::Key::T,
    egui::Key::U,
    egui::Key::V,
    egui::Key::W,
    egui::Key::X,
    egui::Key::Y,
    egui::Key::Z,
    egui::Key::ArrowUp,
    egui::Key::ArrowDown,
    egui::Key::ArrowLeft,
    egui::Key::ArrowRight,
];

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/img/icon.png");
    let image = image::load_from_memory(bytes)
        .expect("failed to load icon")
        .into_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    egui::IconData {
        rgba,
        width,
        height,
    }
}

fn keymap_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".rusted-chip8-keys"))
}

fn load_keymap() -> [egui::Key; 16] {
    let Some(path) = keymap_path() else {
        return DEFAULT_KEYMAP;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return DEFAULT_KEYMAP;
    };
    let mut map = DEFAULT_KEYMAP;
    let mut seen = Vec::new();
    let names: Vec<_> = text.split_whitespace().collect();
    if names.len() != 16 {
        return DEFAULT_KEYMAP;
    }
    for (i, name) in names.iter().enumerate() {
        let Some(&key) = ALLOWED_KEYS
            .iter()
            .find(|&&key| format!("{key:?}") == *name)
        else {
            return DEFAULT_KEYMAP;
        };
        if seen.contains(&key) {
            return DEFAULT_KEYMAP;
        }
        map[i] = key;
        seen.push(key);
    }
    map
}

fn save_keymap(map: &[egui::Key; 16]) -> Result<(), String> {
    let path = keymap_path().ok_or("No s'ha trobat la carpeta personal")?;
    let text = map
        .iter()
        .map(|key| format!("{key:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, text).map_err(|err| format!("No s'han pogut desar les tecles: {err}"))
}

fn key_label(key: egui::Key) -> String {
    match key {
        egui::Key::ArrowUp => "↑".into(),
        egui::Key::ArrowDown => "↓".into(),
        egui::Key::ArrowLeft => "←".into(),
        egui::Key::ArrowRight => "→".into(),
        other => format!("{other:?}").trim_start_matches("Num").to_owned(),
    }
}

const KEYPAD: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rusted CHIP-8 — Debugger")
            .with_icon(load_icon())
            .with_inner_size([900.0, 770.0])
            .with_position([10.0, 70.0])
            .with_min_inner_size([980.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rusted CHIP-8",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = BACKGROUND;
            visuals.window_fill = PANEL;
            visuals.widgets.inactive.bg_fill = HEADER;
            visuals.widgets.inactive.bg_stroke = Stroke::new(0.6, LINE);
            visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.6, LINE);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(217, 228, 236);
            visuals.selection.bg_fill = CURRENT;
            visuals.selection.stroke = Stroke::new(1.0, BLUE);
            // Controls rectangulars, com una aplicació clàssica de macOS/Windows.
            visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
            visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
            visuals.widgets.active.corner_radius = egui::CornerRadius::same(2);
            visuals.widgets.open.corner_radius = egui::CornerRadius::same(2);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(EmulatorApp::new()))
        }),
    )
}

struct EmulatorApp {
    cpu: Cpu,
    rom: Vec<u8>,
    rom_name: String,
    running: bool,
    output_open: bool,
    ips: f32,
    last_frame: Instant,
    cpu_budget: f64,
    timer_budget: f64,
    steps: u64,
    memory_page: usize,
    breakpoints: BTreeSet<u16>,
    breakpoint_text: String,
    message: Option<String>,
    main_keys: [bool; 16],
    output_keys: [bool; 16],
    mouse_keys: [bool; 16],
    keymap: [egui::Key; 16],
    key_binding_target: Option<u8>,
}

impl EmulatorApp {
    fn new() -> Self {
        let mut cpu = Cpu::new();
        cpu._parse(DEMO).expect("La demo integrada és vàlida");
        let mut app = Self {
            cpu,
            rom: DEMO.to_vec(),
            rom_name: "Demo — ABC".into(),
            running: false, // Sempre arrenca PAUSAT.
            output_open: true,
            ips: 700.0,
            last_frame: Instant::now(),
            cpu_budget: 0.0,
            timer_budget: 0.0,
            steps: 0,
            memory_page: 2,
            breakpoints: BTreeSet::new(),
            breakpoint_text: "0200".into(),
            message: None,
            main_keys: [false; 16],
            output_keys: [false; 16],
            mouse_keys: [false; 16],
            keymap: load_keymap(),
            key_binding_target: None,
        };
        if let Some(path) = std::env::args_os().nth(1) {
            app.load_path(Path::new(&path));
        }
        app
    }

    fn reset(&mut self) {
        let mut cpu = Cpu::new();
        match cpu._parse(&self.rom) {
            Ok(()) => {
                self.cpu = cpu;
                self.running = false;
                self.steps = 0;
                self.cpu_budget = 0.0;
                self.timer_budget = 0.0;
                self.main_keys.fill(false);
                self.output_keys.fill(false);
                self.mouse_keys.fill(false);
                self.message = None;
            }
            Err(err) => self.message = Some(err),
        }
        self.last_frame = Instant::now();
    }

    fn load_path(&mut self, path: &Path) {
        match std::fs::read(path) {
            Ok(rom) => {
                let mut cpu = Cpu::new();
                match cpu._parse(&rom) {
                    Ok(()) => {
                        self.cpu = cpu;
                        self.rom = rom;
                        self.rom_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        self.running = false;
                        self.steps = 0;
                        self.cpu_budget = 0.0;
                        self.timer_budget = 0.0;
                        self.memory_page = 2;
                        self.breakpoints.clear();
                        self.main_keys.fill(false);
                        self.output_keys.fill(false);
                        self.mouse_keys.fill(false);
                        self.message = None;
                    }
                    Err(e) => self.message = Some(format!("ROM no vàlida: {e}")),
                }
            }
            Err(e) => self.message = Some(format!("No s'ha pogut obrir la ROM: {e}")),
        }
        self.last_frame = Instant::now();
    }

    fn open_rom(&mut self) {
        self.running = false;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ROM CHIP-8", &["ch8", "c8", "rom", "bin"])
            .pick_file()
        {
            self.load_path(&path);
        }
        self.last_frame = Instant::now();
    }

    fn step(&mut self) {
        self.running = false;
        match self.cpu.decode() {
            Ok(()) => {
                self.steps += 1;
                self.message = None;
            }
            Err(e) => self.message = Some(e),
        }
    }

    fn set_running(&mut self, value: bool) {
        self.running = value;
        self.cpu_budget = 0.0;
        self.timer_budget = 0.0;
        self.last_frame = Instant::now();
        if value {
            self.message = None;
        }
    }

    fn tick(&mut self, elapsed: f64) {
        if !self.running {
            return;
        }
        self.timer_budget += elapsed;
        while self.timer_budget >= 1.0 / 60.0 {
            self.cpu.ui_tick_timers();
            self.timer_budget -= 1.0 / 60.0;
        }
        self.cpu_budget += elapsed * self.ips as f64;
        let cycles = self.cpu_budget.floor().min(200.0) as usize;
        self.cpu_budget -= cycles as f64;
        for _ in 0..cycles {
            if self.breakpoints.contains(&self.cpu.ui_view().pc) {
                self.set_running(false);
                break;
            }
            match self.cpu.decode() {
                Ok(()) => self.steps += 1,
                Err(err) => {
                    self.message = Some(err);
                    self.set_running(false);
                    break;
                }
            }
        }
    }

    fn read_keys(ui: &egui::Ui, target: &mut [bool; 16], keymap: &[egui::Key; 16]) {
        ui.input(|input| {
            if !input.focused
                || input.modifiers.ctrl
                || input.modifiers.command
                || input.modifiers.alt
            {
                target.fill(false);
            } else {
                for (chip8, physical) in keymap.iter().enumerate() {
                    target[chip8] = input.key_down(*physical);
                }
            }
        });
    }

    // La configuració és independent dels opcodes i de la CPU.
    fn capture_key_binding(&mut self, ui: &egui::Ui) {
        let Some(target) = self.key_binding_target else {
            return;
        };

        let captured = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if !modifiers.ctrl && !modifiers.command && !modifiers.alt => Some(*key),
                _ => None,
            })
        });

        let Some(key) = captured else {
            return;
        };
        if key == egui::Key::Escape {
            self.key_binding_target = None;
            return;
        }
        if !ALLOWED_KEYS.contains(&key) {
            return;
        }

        let target = target as usize;
        let previous = self.keymap[target];
        if let Some(other) = self.keymap.iter().position(|&assigned| assigned == key) {
            self.keymap[other] = previous; // Intercanvia les tecles, no hi ha duplicats.
        }
        self.keymap[target] = key;
        self.key_binding_target = None;
        self.main_keys.fill(false);
        self.output_keys.fill(false);
        self.mouse_keys.fill(false);
        if let Err(err) = save_keymap(&self.keymap) {
            self.message = Some(err);
        }
    }

    fn apply_keys(&mut self) {
        for key in 0..16 {
            self.cpu.ui_key(
                key as u8,
                self.main_keys[key] || self.output_keys[key] || self.mouse_keys[key],
            );
        }
    }

    fn shortcuts(&mut self, ui: &egui::Ui) {
        if self.key_binding_target.is_some() {
            return;
        }
        // ⌘1/2/3 per la CPU, ⌘O per obrir una ROM.
        // consume_key evita que les dreceres afectin el teclat CHIP-8.
        let (step, play, pause, open) = ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Num1),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Num2),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Num3),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
            )
        });
        if step {
            self.step();
        }
        if play {
            self.set_running(true);
        }
        if pause {
            self.set_running(false);
        }
        if open {
            self.open_rom();
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            if ui.small_button("Obrir ROM…  ⌘O").clicked() {
                self.open_rom();
            }
            ui.separator();
            if ui.small_button("Step  ⌘1").clicked() {
                self.step();
            }
            if ui.small_button("Run  ⌘2").clicked() {
                self.set_running(true);
            }
            if ui.small_button("Pause  ⌘3").clicked() {
                self.set_running(false);
            }
            if ui.small_button("Reset").clicked() {
                self.reset();
            }
            ui.separator();
            if ui
                .small_button(if self.output_open {
                    "Ocultar vídeo"
                } else {
                    "Mostrar vídeo"
                })
                .clicked()
            {
                self.output_open = !self.output_open;
            }
            ui.separator();
            // Sense slider: selector discret de velocitats fixes.
            ui.label(RichText::new("IPS").monospace().small().color(DIM));
            ui.add_sized(
                [65.0, 19.0],
                egui::DragValue::new(&mut self.ips)
                    .range(1.0..=2500.0)
                    .speed(50.0)
                    .fixed_decimals(0),
            );
            ui.separator();
            let (status, color) = if self.running {
                ("● RUN", GREEN)
            } else {
                ("● PAUSA", PAUSE_RED)
            };
            ui.label(RichText::new(status).monospace().strong().color(color));
        });
    }

    fn output_window(&mut self, main_ui: &egui::Ui) {
        if !self.output_open {
            self.output_keys.fill(false);
            return;
        }
        let ctx = main_ui.ctx().clone();
        let mut close = false;
        self.output_keys.fill(false);
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("chip8_retro_video"),
            egui::ViewportBuilder::default()
                .with_title("CHIP-8 — Video output")
                .with_inner_size([448.0, 224.0])
                .with_position([1000.0, 70.0])
                .with_resizable(true)
                .with_close_button(true)
                .with_maximize_button(true),
            |ui, _class| {
                if ui.input(|i| i.viewport().close_requested()) {
                    close = true;
                    return;
                }
                if self.key_binding_target.is_none() {
                    Self::read_keys(ui, &mut self.output_keys, &self.keymap);
                }
                self.shortcuts(ui);
                let view = self.cpu.ui_view();
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::default()
                            .fill(VIDEO_BACKGROUND)
                            .inner_margin(0),
                    )
                    .show(ui, |ui| draw_video(ui, view.display));
            },
        );
        if close {
            self.output_open = false;
            self.output_keys.fill(false);
        }
    }

    fn debugger(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(7)),
            )
            .show(root, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                self.toolbar(ui);
                hairline(ui);

                let view = self.cpu.ui_view();
                let width = ui.available_width();
                // 16 files de memòria, un peu i una barra de navegació.
                // La resta de l'alçada és per al desassemblador i la CPU.
                let memory_height = 18.0 + 16.0 * 17.0 + 28.0;
                let top_height = (ui.available_height() - memory_height - 27.0).max(190.0);
                let left_width = (width * 0.49).floor();
                let center_width = (width * 0.30).floor();
                let right_width = (width - left_width - center_width - 15.0).max(160.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(width, top_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        ui.allocate_ui_with_layout(
                            egui::vec2(left_width, top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                disassembly(ui, &view, &mut self.breakpoints, top_height);
                            },
                        );
                        vertical_rule(ui, top_height);
                        ui.allocate_ui_with_layout(
                            egui::vec2(center_width, top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                cpu_state(ui, &view, self.steps);
                                ui.add_space(7.0);
                                breakpoint_editor(
                                    ui,
                                    &mut self.breakpoints,
                                    &mut self.breakpoint_text,
                                    &mut self.message,
                                );
                                ui.add_space(6.0);
                                stack(ui, &view, top_height < 265.0);
                            },
                        );
                        vertical_rule(ui, top_height);
                        ui.allocate_ui_with_layout(
                            egui::vec2(right_width, top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                keypad(
                                    ui,
                                    &mut self.mouse_keys,
                                    &self.main_keys,
                                    &self.output_keys,
                                    &self.keymap,
                                );
                                ui.add_space(10.0);
                                key_config_panel(
                                    ui,
                                    &mut self.keymap,
                                    &mut self.key_binding_target,
                                    &mut self.message,
                                    &mut self.main_keys,
                                    &mut self.output_keys,
                                    &mut self.mouse_keys,
                                );
                            },
                        );
                    },
                );

                hairline(ui);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(
                        RichText::new("MEMÒRIA")
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(INK),
                    );
                    if ui.small_button("◀").clicked() {
                        self.memory_page = self.memory_page.saturating_sub(1);
                    }
                    ui.label(
                        RichText::new(format!("{:X}/F", self.memory_page))
                            .monospace()
                            .size(11.5),
                    );
                    if ui.small_button("▶").clicked() {
                        self.memory_page = (self.memory_page + 1).min(15);
                    }
                    if ui.small_button("PC").clicked() {
                        self.memory_page = (view.pc as usize / 256).min(15);
                    }
                    if ui.small_button("I").clicked() {
                        self.memory_page = (view.i as usize / 256).min(15);
                    }
                    ui.separator();
                    ui.label(
                        RichText::new("16 × 16 bytes · clic a PC/I per anar-hi")
                            .size(10.5)
                            .color(DIM),
                    );
                });
                memory(ui, &view, self.memory_page);
                hairline(ui);
                ui.horizontal(|ui| {
                    let status = if let Some(err) = &self.message {
                        format!("ERROR: {err}")
                    } else {
                        format!(
                            "{}    |    {} bytes    |    PC {:04X}    |    {} instruccions",
                            self.rom_name,
                            self.rom.len(),
                            view.pc,
                            self.steps
                        )
                    };
                    ui.label(RichText::new(status).monospace().size(10.5).color(
                        if self.message.is_some() {
                            Color32::from_rgb(148, 72, 64)
                        } else {
                            DIM
                        },
                    ));
                });
            });
    }
}

impl eframe::App for EmulatorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let elapsed = (now - self.last_frame).as_secs_f64().min(0.10);
        self.last_frame = now;

        self.capture_key_binding(ui);
        if self.key_binding_target.is_none() {
            Self::read_keys(ui, &mut self.main_keys, &self.keymap);
        } else {
            self.main_keys.fill(false);
            self.output_keys.fill(false);
            self.mouse_keys.fill(false);
        }
        self.shortcuts(ui);
        self.output_window(ui);
        self.apply_keys();
        self.tick(elapsed);
        self.debugger(ui);
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

// Dues línies són suficients per separar àrees: cap "card" ni marc repetit.
fn hairline(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, LINE));
}

fn vertical_rule(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, height), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        Stroke::new(1.0, LINE),
    );
}

fn small_heading(ui: &mut egui::Ui, name: &str) {
    ui.label(
        RichText::new(name)
            .monospace()
            .strong()
            .size(11.5)
            .color(INK),
    );
}

fn cpu_state(ui: &mut egui::Ui, view: &CpuView<'_>, steps: u64) {
    small_heading(ui, "PROCESSADOR");
    ui.add_space(6.0);
    egui::Grid::new("classic_cpu_state")
        .num_columns(6)
        .spacing([7.0, 5.0])
        .show(ui, |ui| {
            for (name, value, tint) in [
                ("PC", format!("{:04X}", view.pc), BLUE),
                ("I", format!("{:04X}", view.i), INK),
                ("SP", format!("{:02X}", view.sp), INK),
            ] {
                ui.label(RichText::new(name).monospace().size(11.0).color(DIM));
                ui.label(
                    RichText::new(value)
                        .monospace()
                        .strong()
                        .size(12.0)
                        .color(tint),
                );
            }
            ui.end_row();
            for (name, value, tint) in [
                ("DT", format!("{:02X}", view.dlt), GREEN),
                ("ST", format!("{:02X}", view.sdt), GREEN),
                ("#", steps.to_string(), DIM),
            ] {
                ui.label(RichText::new(name).monospace().size(11.0).color(DIM));
                ui.label(RichText::new(value).monospace().size(12.0).color(tint));
            }
            ui.end_row();
        });
    ui.add_space(7.0);
    ui.label(
        RichText::new("REGISTRES   V0–VF")
            .monospace()
            .size(10.5)
            .color(DIM),
    );
    // Taula de valors, sense fitxes de color per a cada registre.
    let col_width = ((ui.available_width() - 8.0) / 4.0).max(41.0);
    for row in 0..4 {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for col in 0..4 {
                let x = row * 4 + col;
                ui.allocate_ui_with_layout(
                    egui::vec2(col_width, 18.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        ui.label(
                            RichText::new(format!("{x:X}:"))
                                .monospace()
                                .size(11.0)
                                .color(DIM),
                        );
                        ui.label(
                            RichText::new(format!("{:02X}", view.v[x]))
                                .monospace()
                                .strong()
                                .size(11.5)
                                .color(byte_color(view.v[x])),
                        );
                    },
                );
            }
        });
    }
}

fn disassembly(
    ui: &mut egui::Ui,
    view: &CpuView<'_>,
    breakpoints: &mut BTreeSet<u16>,
    height: f32,
) {
    let saved_gap = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    small_heading(ui, "DISASSEMBLY");
    ui.add_space(3.0);
    let width = ui.available_width();
    let header_font = egui::FontId::monospace(10.5);
    let (hdr, _) = ui.allocate_exact_size(egui::vec2(width, 19.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(hdr, 0.0, HEADER);
    for (x, title) in [
        (9.0, "BP"),
        (39.0, "ADDR"),
        (101.0, "OP"),
        (164.0, "INSTRUCCIÓ"),
    ] {
        p.text(
            hdr.left_center() + egui::vec2(x, 0.0),
            egui::Align2::LEFT_CENTER,
            title,
            header_font.clone(),
            DIM,
        );
    }

    let count = ((height - 47.0) / 18.0).floor().max(4.0) as usize;
    let before = count / 3;
    let start = ((view.pc as usize).saturating_sub(before * 2) & !1).min(4094);
    for offset in 0..count {
        let addr = start + offset * 2;
        if addr + 1 >= view.memory.len() {
            break;
        }
        let op = (u16::from(view.memory[addr]) << 8) | u16::from(view.memory[addr + 1]);
        let here = view.pc as usize == addr;
        let bp = breakpoints.contains(&(addr as u16));
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::click());
        let p = ui.painter();
        if here {
            p.rect_filled(rect, 0.0, CURRENT);
        } else if offset % 2 == 1 {
            p.rect_filled(rect, 0.0, PANEL);
        }
        if here {
            p.rect_filled(
                egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, 18.0)),
                0.0,
                BLUE,
            );
        }
        let font = egui::FontId::monospace(11.5);
        let y = rect.center().y;
        p.text(
            egui::pos2(rect.left() + 11.0, y),
            egui::Align2::LEFT_CENTER,
            if bp { "●" } else { "·" },
            font.clone(),
            if bp { AMBER } else { LINE },
        );
        p.text(
            egui::pos2(rect.left() + 39.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{addr:04X}"),
            font.clone(),
            if here { BLUE } else { DIM },
        );
        p.text(
            egui::pos2(rect.left() + 101.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{op:04X}"),
            font.clone(),
            INK,
        );
        p.text(
            egui::pos2(rect.left() + 164.0, y),
            egui::Align2::LEFT_CENTER,
            disasm(op),
            font,
            if here { BLUE } else { INK },
        );
        if response
            .on_hover_text("Clica per alternar breakpoint")
            .clicked()
        {
            if !breakpoints.insert(addr as u16) {
                breakpoints.remove(&(addr as u16));
            }
        }
    }
    ui.spacing_mut().item_spacing.y = saved_gap;
}

fn breakpoint_editor(
    ui: &mut egui::Ui,
    breakpoints: &mut BTreeSet<u16>,
    input: &mut String,
    message: &mut Option<String>,
) {
    small_heading(ui, "BREAKPOINTS");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_sized(
            [62.0, 20.0],
            egui::TextEdit::singleline(input).font(egui::TextStyle::Monospace),
        );
        if ui.small_button("Afegir").clicked() {
            match u16::from_str_radix(
                input
                    .trim()
                    .trim_start_matches("0x")
                    .trim_start_matches("0X"),
                16,
            ) {
                Ok(addr) if addr < 4096 => {
                    breakpoints.insert(addr);
                }
                _ => *message = Some("Breakpoint invàlid: 0000–0FFF".into()),
            }
        }
        if ui.small_button("Netejar").clicked() {
            breakpoints.clear();
        }
    });
    let entries = breakpoints
        .iter()
        .take(6)
        .map(|b| format!("{b:04X}"))
        .collect::<Vec<_>>()
        .join("  ");
    ui.label(
        RichText::new(if entries.is_empty() {
            "Cap breakpoint"
        } else {
            &entries
        })
        .monospace()
        .size(10.5)
        .color(DIM),
    );
}

// Configuració en línia: aprofita l'espai buit sota el keypad.
// Els 16 botons mostren «tecla CHIP-8 : tecla física».
fn key_config_panel(
    ui: &mut egui::Ui,
    keymap: &mut [egui::Key; 16],
    target: &mut Option<u8>,
    message: &mut Option<String>,
    main_keys: &mut [bool; 16],
    output_keys: &mut [bool; 16],
    mouse_keys: &mut [bool; 16],
) {
    small_heading(ui, "ASSIGNACIÓ DE TECLES");
    ui.add_space(3.0);
    ui.label(
        RichText::new("Clica un valor i prem A–Z, 0–9 o una fletxa")
            .size(9.5)
            .color(DIM),
    );
    ui.add_space(4.0);

    let width = ((ui.available_width() - 9.0) / 4.0).max(32.0);
    for row in KEYPAD {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for chip8 in row {
                let waiting = *target == Some(chip8);
                let label = if waiting {
                    format!("{chip8:X}:…")
                } else {
                    format!("{chip8:X}:{}", key_label(keymap[chip8 as usize]))
                };
                let text = RichText::new(label)
                    .monospace()
                    .size(10.5)
                    .color(if waiting { BLUE } else { INK });
                let text = if waiting { text.strong() } else { text };
                let response = ui.add_sized(
                    [width, 20.0],
                    egui::Label::new(text).sense(egui::Sense::click()),
                );
                if response.clicked() {
                    *target = Some(chip8);
                    main_keys.fill(false);
                    output_keys.fill(false);
                    mouse_keys.fill(false);
                }
            }
        });
    }
    ui.add_space(4.0);
    if let Some(key) = *target {
        ui.label(
            RichText::new(format!("{key:X}: prem tecla · Esc cancel·la"))
                .monospace()
                .size(10.0)
                .color(BLUE),
        );
    } else {
        ui.label(RichText::new("Desat automàtic").size(10.0).color(DIM));
    }
    if ui.small_button("Valors per defecte").clicked() {
        *keymap = DEFAULT_KEYMAP;
        *target = None;
        main_keys.fill(false);
        output_keys.fill(false);
        mouse_keys.fill(false);
        if let Err(err) = save_keymap(keymap) {
            *message = Some(err);
        }
    }
}

fn keypad(
    ui: &mut egui::Ui,
    mouse: &mut [bool; 16],
    main: &[bool; 16],
    output: &[bool; 16],
    keymap: &[egui::Key; 16],
) {
    small_heading(ui, "KEYPAD");
    ui.add_space(6.0);
    let previous_mouse = *mouse;
    mouse.fill(false);
    for row in KEYPAD {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for key in row {
                let active =
                    main[key as usize] || output[key as usize] || previous_mouse[key as usize];
                let label = RichText::new(format!("{key:X}"))
                    .monospace()
                    .size(12.5)
                    .color(if active { KEY_ACTIVE } else { INK });
                let label = if active { label.strong() } else { label };
                let button = egui::Button::new(label)
                    .min_size(egui::vec2(33.0, 27.0))
                    .fill(if active { KEY_ACTIVE_FILL } else { PANEL })
                    .stroke(Stroke::new(
                        if active { 2.4 } else { 0.7 },
                        if active { KEY_ACTIVE } else { LINE },
                    ));
                let response = ui
                    .add(button)
                    .on_hover_text(format!("Tecla física: {}", key_label(keymap[key as usize])));
                let pressed = response.is_pointer_button_down_on();
                mouse[key as usize] = pressed;
                if pressed && !active {
                    // Feedback immediat durant el primer fotograma del clic.
                    ui.painter().rect_stroke(
                        response.rect,
                        2.0,
                        Stroke::new(2.4, KEY_ACTIVE),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        });
    }
}

fn stack(ui: &mut egui::Ui, view: &CpuView<'_>, compact: bool) {
    let visible = if compact { 2 } else { 4 };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("STACK")
                .monospace()
                .strong()
                .size(11.0)
                .color(INK),
        );
        if view.sp == 0 {
            ui.label(RichText::new("—").monospace().color(DIM));
        } else {
            for ix in ((view.sp as usize).saturating_sub(visible)..view.sp as usize).rev() {
                ui.label(
                    RichText::new(format!("{:04X}", view.stack[ix]))
                        .monospace()
                        .size(10.5)
                        .color(INK),
                );
            }
        }
    });
}

fn memory(ui: &mut egui::Ui, view: &CpuView<'_>, page: usize) {
    // Hex dump continu: la zona ASCII ocupa aproximadament un terç de l'ample.
    let saved_gap = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    const ROW_H: f32 = 17.0;
    let w = ui.available_width();
    let start_x = 60.0;
    let ascii_reserved = (w * 0.31).max(235.0);
    let cell = ((w - start_x - ascii_reserved - 13.0) / 16.0).clamp(25.0, 42.0);
    let ascii_x = start_x + cell * 16.0 + 13.0;
    let ascii_cell = ((w - ascii_x - 8.0) / 16.0).max(7.0);
    let font = egui::FontId::monospace(11.0);
    let ascii_font = egui::FontId::monospace(11.5);

    let (head, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    ui.painter().rect_filled(head, 0.0, HEADER);
    ui.painter().text(
        egui::pos2(head.left() + 4.0, head.center().y),
        egui::Align2::LEFT_CENTER,
        "ADDR",
        egui::FontId::monospace(10.5),
        DIM,
    );
    for col in 0..16 {
        ui.painter().text(
            egui::pos2(
                head.left() + start_x + cell * (col as f32 + 0.5),
                head.center().y,
            ),
            egui::Align2::CENTER_CENTER,
            format!("{col:02X}"),
            egui::FontId::monospace(10.5),
            DIM,
        );
    }
    ui.painter().text(
        egui::pos2(head.left() + ascii_x, head.center().y),
        egui::Align2::LEFT_CENTER,
        "ASCII",
        egui::FontId::monospace(10.5),
        DIM,
    );
    ui.painter().line_segment(
        [
            egui::pos2(head.left() + ascii_x - 7.0, head.top()),
            egui::pos2(head.left() + ascii_x - 7.0, head.bottom()),
        ],
        Stroke::new(0.8, LINE),
    );

    for row in 0..16 {
        let address = page * 256 + row * 16;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::hover());
        let painter = ui.painter();
        if row % 2 == 1 {
            painter.rect_filled(rect, 0.0, PANEL);
        }
        painter.text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{address:04X}"),
            font.clone(),
            if view.pc as usize / 16 == address / 16 {
                BLUE
            } else {
                DIM
            },
        );

        painter.line_segment(
            [
                egui::pos2(rect.left() + ascii_x - 7.0, rect.top()),
                egui::pos2(rect.left() + ascii_x - 7.0, rect.bottom()),
            ],
            Stroke::new(0.5, LINE),
        );

        for col in 0..16 {
            let addr = address + col;
            let value = view.memory[addr];
            let center = egui::pos2(
                rect.left() + start_x + cell * (col as f32 + 0.5),
                rect.center().y,
            );
            if addr == view.pc as usize || addr == view.pc as usize + 1 {
                painter.rect_filled(
                    egui::Rect::from_center_size(center, egui::vec2(cell - 2.0, ROW_H - 1.0)),
                    0.0,
                    CURRENT,
                );
            } else if addr == view.i as usize {
                painter.rect_filled(
                    egui::Rect::from_center_size(center, egui::vec2(cell - 2.0, ROW_H - 1.0)),
                    0.0,
                    Color32::from_rgb(247, 238, 220),
                );
            } else if value != 0 {
                painter.rect_filled(
                    egui::Rect::from_center_size(center, egui::vec2(cell - 4.0, ROW_H - 2.0)),
                    0.0,
                    byte_tint(value),
                );
            }
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                format!("{value:02X}"),
                font.clone(),
                byte_color(value),
            );

            // El text ASCII té una columna per byte, repartida en tot l'espai disponible.
            // A diferència del visor hexadecimal, no hi ha blocs pintats al darrere.
            let ascii_char = if (32..=126).contains(&value) {
                value as char
            } else {
                '.'
            };
            let ascii_center = egui::pos2(
                rect.left() + ascii_x + ascii_cell * (col as f32 + 0.5),
                rect.center().y,
            );
            painter.text(
                ascii_center,
                egui::Align2::CENTER_CENTER,
                ascii_char.to_string(),
                ascii_font.clone(),
                if addr == view.pc as usize {
                    BLUE
                } else if value == 0 {
                    DIM
                } else {
                    INK
                },
            );
        }
        response.on_hover_text(format!("{address:04X}–{:04X}", address + 15));
    }
    ui.spacing_mut().item_spacing.y = saved_gap;
}

// Una paleta discreta aplicada al text de cada valor hexadecimal.
fn byte_color(value: u8) -> Color32 {
    const COLORS: [Color32; 8] = [
        Color32::from_rgb(69, 91, 116),
        Color32::from_rgb(83, 106, 82),
        Color32::from_rgb(117, 88, 103),
        Color32::from_rgb(124, 98, 67),
        Color32::from_rgb(83, 99, 124),
        Color32::from_rgb(107, 91, 120),
        Color32::from_rgb(70, 108, 106),
        Color32::from_rgb(132, 93, 82),
    ];
    if value == 0 {
        DIM
    } else {
        COLORS[(value as usize * 13) % COLORS.len()]
    }
}

fn byte_tint(value: u8) -> Color32 {
    const TINTS: [Color32; 8] = [
        Color32::from_rgb(238, 243, 247),
        Color32::from_rgb(240, 244, 238),
        Color32::from_rgb(246, 240, 244),
        Color32::from_rgb(246, 243, 236),
        Color32::from_rgb(239, 243, 247),
        Color32::from_rgb(243, 239, 247),
        Color32::from_rgb(238, 246, 244),
        Color32::from_rgb(247, 241, 238),
    ];
    TINTS[(value as usize * 13) % TINTS.len()]
}

fn draw_video(ui: &mut egui::Ui, pixels: &[bool; 2048]) {
    let rect = ui.available_rect_before_wrap();
    let scale = (rect.width() / 64.0).min(rect.height() / 32.0);
    if scale <= 0.0 {
        return;
    }
    let size = egui::vec2(64.0 * scale, 32.0 * scale);
    let top_left = rect.center() - size * 0.5;
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, VIDEO_BACKGROUND);
    for y in 0..32 {
        for x in 0..64 {
            if pixels[y * 64 + x] {
                let pos = top_left + egui::vec2(x as f32 * scale, y as f32 * scale);
                painter.rect_filled(
                    egui::Rect::from_min_size(pos, egui::vec2(scale, scale)),
                    0.0,
                    VIDEO_COLOR,
                );
            }
        }
    }
}

fn disasm(op: u16) -> String {
    let x = (op >> 8) & 0xF;
    let y = (op >> 4) & 0xF;
    let n = op & 0xF;
    let kk = op & 0xFF;
    let nnn = op & 0xFFF;
    match op >> 12 {
        0x0 => match op {
            0x00E0 => "CLS".into(),
            0x00EE => "RET".into(),
            _ => "???".into(),
        },
        0x1 => format!("JP      {nnn:03X}"),
        0x2 => format!("CALL    {nnn:03X}"),
        0x3 => format!("SE      V{x:X}, {kk:02X}"),
        0x4 => format!("SNE     V{x:X}, {kk:02X}"),
        0x5 if n == 0 => format!("SE      V{x:X}, V{y:X}"),
        0x6 => format!("LD      V{x:X}, {kk:02X}"),
        0x7 => format!("ADD     V{x:X}, {kk:02X}"),
        0x8 => match n {
            0x0 => format!("LD      V{x:X}, V{y:X}"),
            0x1 => format!("OR      V{x:X}, V{y:X}"),
            0x2 => format!("AND     V{x:X}, V{y:X}"),
            0x3 => format!("XOR     V{x:X}, V{y:X}"),
            0x4 => format!("ADD     V{x:X}, V{y:X}"),
            0x5 => format!("SUB     V{x:X}, V{y:X}"),
            0x6 => format!("SHR     V{x:X}"),
            0x7 => format!("SUBN    V{x:X}, V{y:X}"),
            0xE => format!("SHL     V{x:X}"),
            _ => "???".into(),
        },
        0x9 if n == 0 => format!("SNE     V{x:X}, V{y:X}"),
        0xA => format!("LD      I, {nnn:03X}"),
        0xB => format!("JP      V0, {nnn:03X}"),
        0xC => format!("RND     V{x:X}, {kk:02X}"),
        0xD => format!("DRW     V{x:X}, V{y:X}, {n:X}"),
        0xE => match kk {
            0x9E => format!("SKP     V{x:X}"),
            0xA1 => format!("SKNP    V{x:X}"),
            _ => "???".into(),
        },
        0xF => match kk {
            0x07 => format!("LD      V{x:X}, DT"),
            0x0A => format!("LD      V{x:X}, K"),
            0x15 => format!("LD      DT, V{x:X}"),
            0x18 => format!("LD      ST, V{x:X}"),
            0x1E => format!("ADD     I, V{x:X}"),
            0x29 => format!("LD      F, V{x:X}"),
            0x33 => format!("LD      B, V{x:X}"),
            0x55 => format!("LD      [I], V{x:X}"),
            0x65 => format!("LD      V{x:X}, [I]"),
            _ => "???".into(),
        },
        _ => "???".into(),
    }
}
