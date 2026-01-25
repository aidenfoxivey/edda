#![allow(dead_code)]

//! https://docs.rs/meshtastic/latest/meshtastic/
//! https://docs.rs/sqlite/latest/sqlite/
//! https://docs.rs/ratatui/latest/ratatui/
//!
//! A few goals for the project:
//! - graceful degradation on disconnection
//! - clear UI for sending messages
//! - support direct messages

use std::fs::OpenOptions;
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use env_logger::Builder;
use tokio::sync::mpsc;

use crate::tui::App;

mod mesh;
mod router;
mod tui;
mod types;

fn setup_logger() {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let target = Box::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{}_app.log", since_the_epoch.as_secs()))
            .expect("Failed to open log file"),
    );

    Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();
}

fn main() -> Result<()> {
    setup_logger();
    color_eyre::install()?;
    let (ui_tx, ui_rx) = mpsc::channel(100);
    let (tx, rx) = mpsc::channel(100);

    // Run a seperate thread that listens to the Meshtastic interface.
    std::thread::spawn(move || {
        if let Err(e) = mesh::run_meshtastic(ui_rx, tx) {
            eprintln!("Meshtastic thread error: {}", e);
        }
    });

    // Generate the terminal handlers and run the Ratatui application.
    let mut terminal = ratatui::init();
    let mut app = App::new(rx, ui_tx);
    // Take a receiver to transport information between the Meshtastic thread and the terminal thread.
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}
