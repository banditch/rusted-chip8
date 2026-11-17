mod cpu;
mod display;
mod keys;
mod time;
mod ui;

fn main() -> eframe::Result<()> {
    ui::run()
}
