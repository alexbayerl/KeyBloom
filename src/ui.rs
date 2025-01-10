//! Terminal User Interface (TUI) for KeyBloom.
//!
//! This file defines the interactive menu for editing the
//! `Config` fields. It uses `ratatui` (based on `tui-rs`) and
//! `crossterm` for handling user input in a terminal environment.

use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering}; // NEW

use crate::config::Config;
use crate::sync_loop::{start_sync_loop, SyncStatus};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Alignment, Constraint, Direction};
use ratatui::style::{Color as RColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};
use std::thread;

// Define a new error type that implements Send + Sync + 'static
type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Represents the TUI's input mode for editing a configuration field or just navigating.
#[derive(Debug, PartialEq, Clone)]
pub enum InputMode {
    Normal,
    Editing,
    Syncing,
}

/// The main application state for the TUI.
pub struct App {
    /// The active configuration for KeyBloom.
    pub config: Config,
    /// List of configuration options in textual form.
    pub options: Vec<&'static str>,
    /// Descriptions for each configuration option to display to the user.
    pub descriptions: Vec<&'static str>,
    /// Indicates whether we're in `Normal`, `Editing`, or `Syncing` mode.
    pub input_mode: InputMode,
    /// The temporary buffer that holds user input when editing.
    pub input: String,
    /// Stores the currently selected item in the list for navigation.
    pub list_state: ratatui::widgets::ListState,
    /// Indicates whether the UI needs to be redrawn.
    pub dirty: bool,
    /// Shared synchronization status (updated by the sync loop).
    pub sync_status: Arc<Mutex<SyncStatus>>,
    /// Handle to the running sync loop, if any.
    pub sync_handle: Option<thread::JoinHandle<()>>,
    /// Shared stop signal to gracefully terminate the sync loop.
    pub stop_signal: Arc<AtomicBool>, // NEW
}

impl App {
    /// Create a new `App` instance from a given `Config`.
    pub fn new(config: Config) -> Self {
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));

        App {
            config,
            options: vec![
                "Number of LEDs",
                "Transition Steps",
                "Transition Delay (ms)",
                "Frame Delay (ms)",
                "Color Change Threshold",
                "Brightness Factor",
                "Saturation Factor",
                "Debounce Duration (ms)",
                "OpenRGB Host",
                "OpenRGB Port",
                "Device Name",
                "Monitor Index",
                "Save and Sync",
            ],
            descriptions: vec![
                "Set the number of LEDs on your device.",
                "Define how many steps the color transition should take.",
                "Specify the delay (ms) between each transition step.",
                "Set the delay (ms) between each frame capture.",
                "Threshold for significant color changes (0.0-1.0).",
                "Factor to adjust overall brightness (larger = brighter).",
                "Factor to adjust color saturation (larger = more vibrant).",
                "Minimum duration (ms) between transitions to prevent rapid changes.",
                "Hostname or IP of the OpenRGB server.",
                "Port number of the OpenRGB server.",
                "Name of the OpenRGB device to control.",
                "Index of the monitor to capture (0-based).",
                "Save current configuration and exit the menu.",
            ],
            input_mode: InputMode::Normal,
            input: String::new(),
            list_state,
            dirty: true,
            sync_status: Arc::new(Mutex::new(SyncStatus::default())),
            sync_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false)), // NEW
        }
    }

    /// Move selection down in the options list.
    pub fn next(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            let next = if selected >= self.options.len() - 1 { 0 } else { selected + 1 };
            self.list_state.select(Some(next));
            self.dirty = true;
        }
    }

    /// Move selection up in the options list.
    pub fn previous(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            let prev = if selected == 0 {
                self.options.len() - 1
            } else {
                selected - 1
            };
            self.list_state.select(Some(prev));
            self.dirty = true;
        }
    }

    /// Toggle between editing the currently selected field and normal navigation.
    ///
    /// When toggling to `Editing`, the current config value is loaded into `self.input`.
    /// When toggling back to `Normal`, `self.input` is cleared.
    pub fn toggle_edit(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::Normal => InputMode::Editing,
            InputMode::Editing => InputMode::Normal,
            InputMode::Syncing => InputMode::Syncing,
        };
        if self.input_mode == InputMode::Editing {
            let selected = self.list_state.selected().unwrap_or(0);
            self.input = match selected {
                0 => self.config.num_leds.to_string(),
                1 => self.config.transition_steps.to_string(),
                2 => self.config.transition_delay_ms.to_string(),
                3 => self.config.frame_delay_ms.to_string(),
                4 => self.config.color_change_threshold.to_string(),
                5 => self.config.brightness_factor.to_string(),
                6 => self.config.saturation_factor.to_string(),
                7 => self.config.debounce_duration_ms.to_string(),
                8 => self.config.openrgb_host.clone(),
                9 => self.config.openrgb_port.to_string(),
                10 => self.config.device_name.clone(),
                11 => self.config.monitor_index.to_string(),
                _ => "".to_string(),
            };
        } else if self.input_mode == InputMode::Normal {
            // Clear input if returning from editing
            self.input.clear();
        }
        self.dirty = true;
    }

    /// Update the `config` with the contents of `self.input` for the selected option.
    ///
    /// Tries to parse numeric fields or assigns for string fields. If parsing fails,
    /// the old value is retained.
    pub fn update_config(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            match selected {
                0 => {
                    self.config.num_leds = self.input.parse().unwrap_or(self.config.num_leds);
                }
                1 => {
                    self.config.transition_steps =
                        self.input.parse().unwrap_or(self.config.transition_steps);
                }
                2 => {
                    self.config.transition_delay_ms =
                        self.input.parse().unwrap_or(self.config.transition_delay_ms);
                }
                3 => {
                    self.config.frame_delay_ms =
                        self.input.parse().unwrap_or(self.config.frame_delay_ms);
                }
                4 => {
                    self.config.color_change_threshold =
                        self.input.parse().unwrap_or(self.config.color_change_threshold);
                }
                5 => {
                    self.config.brightness_factor =
                        self.input.parse().unwrap_or(self.config.brightness_factor);
                }
                6 => {
                    self.config.saturation_factor =
                        self.input.parse().unwrap_or(self.config.saturation_factor);
                }
                7 => {
                    self.config.debounce_duration_ms =
                        self.input.parse().unwrap_or(self.config.debounce_duration_ms);
                }
                8 => {
                    self.config.openrgb_host = self.input.clone();
                }
                9 => {
                    self.config.openrgb_port =
                        self.input.parse().unwrap_or(self.config.openrgb_port);
                }
                10 => {
                    self.config.device_name = self.input.clone();
                }
                11 => {
                    self.config.monitor_index =
                        self.input.parse().unwrap_or(self.config.monitor_index);
                }
                _ => {}
            }
        }
        self.dirty = true;
    }

    /// Start the actual sync loop in background (spawning a new thread with its own Tokio runtime).
    pub fn start_sync(&mut self) {
        // Reset to false in case we had a previous run
        self.stop_signal.store(false, Ordering::Relaxed); // NEW

        let config = self.config.clone();
        let sync_status = Arc::clone(&self.sync_status);
        let stop_signal = Arc::clone(&self.stop_signal); // NEW

        // Spawn the sync loop in a new thread to avoid Send requirement
        let handle = std::thread::spawn(move || {
            // Create a new Tokio runtime for the spawned thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            // Run the async sync loop within the runtime
            rt.block_on(async {
                if let Err(err) = start_sync_loop(&config, sync_status, stop_signal).await { // MODIFIED
                    eprintln!("Error in sync loop: {err}");
                }
            });
        });
        self.sync_handle = Some(handle);

        // Switch to sync mode
        self.input_mode = InputMode::Syncing;
        self.dirty = true;
    }

    /// Abort the sync loop (if running) and return to the normal mode.
    pub fn stop_sync(&mut self) {
        // Tell the sync loop to break from its while-loop
        self.stop_signal.store(true, Ordering::Relaxed); // NEW

        if let Some(handle) = self.sync_handle.take() {
            // This will now return quickly, because the sync loop sees stop_signal == true
            handle.join().unwrap_or_else(|e| {
                eprintln!("Failed to join sync thread: {:?}", e);
            });
        }

        self.input_mode = InputMode::Normal;
        self.dirty = true;
    }
}

/// Renders the main TUI layout onto the frame.
///
/// # Arguments
///
/// * `f` - The frame to draw onto.
/// * `app` - The current state of the TUI application.
pub fn ui(f: &mut Frame<'_>, app: &mut App) {
    match app.input_mode {
        InputMode::Normal | InputMode::Editing => render_menu(f, app),
        InputMode::Syncing => render_sync_screen(f, app),
    }
}

fn render_menu(f: &mut Frame<'_>, app: &mut App) {
    let area = f.size();
    let chunks = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);

    let ascii_art = r#"         _  __          ____  _                       
        | |/ /___ _   _| __ )| | ___   ___  _ __ ___  
        | ' // _ \ | | |  _ \| |/ _ \ / _ \| '_ ` _ \ 
        | . \  __/ |_| | |_) | | (_) | (_) | | | | | |
        |_|\_\___|\__, |____/|_|\___/ \___/|_| |_| |_|
                  |___/                               "#;

    let header_block = Block::default().borders(Borders::NONE);
    let header_paragraph = Paragraph::new(ascii_art)
        .block(header_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(RColor::Yellow));
    f.render_widget(header_paragraph, chunks[0]);

    // Configuration options list
    let items: Vec<ListItem> = app
        .options
        .iter()
        .map(|opt| ListItem::new(*opt).style(
            Style::default()
                .fg(RColor::White)
                .add_modifier(Modifier::BOLD),
        ))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Configuration Options")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title_alignment(Alignment::Center),
        )
        .highlight_style(
            Style::default()
                .fg(RColor::Black)
                .bg(RColor::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Description of currently selected option
    let selected = app.list_state.selected().unwrap_or(0);
    let description = if selected < app.descriptions.len() {
        app.descriptions[selected]
    } else {
        ""
    };
    let desc_block = Block::default()
        .title("Option Description")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title_alignment(Alignment::Center);
    let desc_paragraph = Paragraph::new(description)
        .block(desc_block)
        .style(Style::default().fg(RColor::LightBlue))
        .alignment(Alignment::Left);
    f.render_widget(desc_paragraph, chunks[2]);

    // Input/edit area or Instructions
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    if app.input_mode == InputMode::Editing {
        let editing_block = input_block
            .clone()
            .title("Edit Value")
            .title_alignment(Alignment::Center);
        let input_widget = Paragraph::new(app.input.as_str())
            .block(editing_block)
            .style(Style::default().fg(RColor::Green))
            .alignment(Alignment::Left);
        f.render_widget(input_widget, chunks[3]);

        // Place the cursor at the end of the input
        let cursor_x = chunks[3].x + app.input.len() as u16 + 1;
        let cursor_y = chunks[3].y + 1;
        f.set_cursor(cursor_x, cursor_y);
    } else {
        let help_block = input_block
            .clone()
            .title("Instructions")
            .title_alignment(Alignment::Center);
        let info_text = "Press 'q' to exit. Use ↑↓ to navigate. Press Enter to edit.";
        let info = Paragraph::new(info_text)
            .block(help_block)
            .style(Style::default().fg(RColor::Gray))
            .alignment(Alignment::Center);
        f.render_widget(info, chunks[3]);
    }

    // Author signature
    let author_paragraph = Paragraph::new("Alexander Bayerl | With ❤️ from Austria")
        .style(Style::default().fg(RColor::Rgb(255, 214, 0)))
        .alignment(Alignment::Right);
    f.render_widget(author_paragraph, chunks[5]);
}

fn render_sync_screen(f: &mut Frame<'_>, app: &mut App) {
    let sync_status = app.sync_status.lock().unwrap();

    // Define layout
    let chunks = ratatui::layout::Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(5),
        ])
        .split(f.size());

    // Header
    let header = Paragraph::new("🔄 Synchronization in Progress")
        .style(Style::default().fg(RColor::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Body - Display current colors
    let colors = &sync_status.current_colors;
    let color_blocks: Vec<ListItem> = colors
        .iter()
        .enumerate()
        .map(|(i, color)| {
            let line = Line::from(vec![
                Span::styled(format!("LED {}: ", i + 1), Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("     ", Style::default().bg(RColor::Rgb(color.r, color.g, color.b))),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(color_blocks)
        .block(Block::default().title("Current LED Colors").borders(Borders::ALL))
        .style(Style::default());
    f.render_widget(list, chunks[1]);

    // Footer with controls
    let footer = Paragraph::new("Press 'm' to return to Menu | 'q' to Quit")
        .style(Style::default().fg(RColor::Gray))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

/// Runs the TUI application loop, handling events and rendering.
///
/// # Arguments
///
/// * `terminal` - A mutable reference to a `Terminal`.
/// * `app` - A mutable reference to the TUI application state.
///
/// # Returns
///
/// An `AnyError` if an error occurs during the run.
pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), AnyError> {
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
    
        if app.dirty || last_tick.elapsed() >= tick_rate {
            terminal.draw(|f| ui(f, app))?;
            app.dirty = false;
            last_tick = now;
        }
    
        if event::poll(timeout)? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let key_char = match key.code {
                        KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
                        _ => None,
                    };
    
                    if let Some(c) = key_char {
                        match c {
                            'm' => {
                                if app.input_mode == InputMode::Syncing {
                                    app.stop_sync();
                                } else {
                                    // Define behavior for 'm' in other modes if needed
                                }
                                continue; // Skip further processing
                            }
                            'q' => {
                                if app.input_mode == InputMode::Syncing {
                                    app.stop_sync();
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
    
                    // Handle other keys based on input mode
                    match app.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Down => {
                                    app.next();
                                }
                                KeyCode::Up => {
                                    app.previous();
                                }
                                KeyCode::Enter => {
                                    if let Some(selected) = app.list_state.selected() {
                                        // "Save and Sync" is the last option
                                        if selected == app.options.len() - 1 {
                                            // Attempt to save configuration
                                            match app.config.save() {
                                                Ok(_) => {
                                                    eprintln!("Configuration saved successfully.");
                                                    // Now start the sync
                                                    app.start_sync();
                                                }
                                                Err(err) => {
                                                    eprintln!("Failed to save configuration: {}", err);
                                                }
                                            }
                                        } else {
                                            app.toggle_edit();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        InputMode::Editing => {
                            match key.code {
                                KeyCode::Enter => {
                                    app.update_config();
                                    app.toggle_edit();
                                }
                                KeyCode::Char(c) => {
                                    app.input.push(c);
                                    app.dirty = true;
                                }
                                KeyCode::Backspace => {
                                    app.input.pop();
                                    app.dirty = true;
                                }
                                KeyCode::Esc => {
                                    app.toggle_edit();
                                }
                                _ => {}
                            }
                        }
                        InputMode::Syncing => {
                            // Handle other keys if necessary
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Launches the TUI menu in raw mode and restores the terminal upon exit.
///
/// # Arguments
///
/// * `config` - A mutable reference to the current KeyBloom configuration.
pub async fn show_menu(config: &mut Config) -> io::Result<()> {
    let mut app = App::new(config.clone());

    // Start up the TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let run_result = match run_app(&mut terminal, &mut app).await {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!("Error running TUI menu: {}", err);
            Err(io::Error::new(io::ErrorKind::Other, err))
        }
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Abort sync if it's running
    app.stop_sync();

    // Reload config from disk if user selected "Save and Sync"
    *config = Config::load();

    run_result
}
