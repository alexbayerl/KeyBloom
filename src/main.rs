//! Main entry point for KeyBloom.
//!
//! This file sets up asynchronous runtime with `tokio`,
//! loads or creates a default configuration, and presents a TUI menu
//! for configuration editing. After the user exits the menu,
//! the sync loop starts.

mod color_utils;
mod config;
mod sync_loop;
mod ui;

use crate::config::Config;
use crate::sync_loop::{start_sync_loop, SyncLoopExit};
use crate::ui::show_menu;
use std::error::Error;

/// Asynchronous main function for KeyBloom.
///
/// Launches the TUI menu, lets the user configure settings, and then starts
/// the synchronization loop that captures screen colors and updates the
/// OpenRGB device in real-time.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load or create default config
    let mut config = Config::load();

    loop {
        // Show TUI menu for editing config
        if let Err(err) = show_menu(&mut config) {
            eprintln!("Menu error: {err}");
        }

        // Start the sync loop using the (possibly updated) configuration
        match start_sync_loop(&config).await? {
            SyncLoopExit::ReturnToMenu => {
                continue;
            }
            SyncLoopExit::Quit => {
                break;
            }
        }
    }
    Ok(())
}