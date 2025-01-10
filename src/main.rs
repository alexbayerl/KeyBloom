//! Main entry point for KeyBloom.
//!
//! This file sets up the tokio runtime and loads or creates a default configuration,
//! then launches our TUI menu.

mod color_utils;
mod config;
mod sync_loop;
mod ui;

use crate::config::Config;
use crate::ui::show_menu;

// Define a new error type that implements Send + Sync + 'static
type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    // Load or create default config
    let mut config = Config::load();

    // Launch the TUI menu (which can handle "Save and Sync" and the sync screen)
    if let Err(err) = show_menu(&mut config).await {
        eprintln!("Error running TUI menu: {err}");
    }

    Ok(())
}
