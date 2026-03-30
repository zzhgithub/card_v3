mod app;
mod cursor;
mod deck_ui;
mod game_setup;
mod input;
mod matchmaker_client;
mod network;
mod ui;

use std::fs;

use anyhow::Result;
use app::App;

fn main() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    fs::create_dir_all("logs")?;
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(|| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/card-tui.log")
                .expect("open logs/card-tui.log")
        })
        .init();

    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();
    result
}
