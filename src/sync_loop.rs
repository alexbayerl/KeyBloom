//! Core logic for capturing screen colors and updating the OpenRGB device.
//!
//! The `start_sync_loop` function handles the following:
//! 1. Connect to the OpenRGB server.
//! 2. Identify the chosen device (keyboard, etc.).
//! 3. Capture the screen from the selected monitor.
//! 4. Compute average colors across screen segments.
//! 5. Transition the keyboard LEDs smoothly to those colors.
//!
//! The loop continues until the user presses 'm' to return to menu or 'q' to quit.

use crate::color_utils::*;
use crate::config::Config;
use image::RgbaImage;
use openrgb::{data::Color, OpenRGB, OpenRGBError};
use palette::Srgb;
use rayon::prelude::*; // For parallel iterators
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use xcap::Monitor;

/// Return signals for the sync loop indicating if we should quit or return to menu.
pub enum SyncLoopExit {
    /// Return the user to the menu.
    ReturnToMenu,
    /// Quit the entire application.
    Quit,
}

/// The main synchronization loop.
///
/// This function connects to the OpenRGB server, finds the device specified by the user,
/// selects the desired monitor for screen capture, and continuously updates the device LEDs
/// based on the average color of different vertical segments of the screen.
///
/// # Arguments
///
/// * `config` - A reference to the current KeyBloom configuration.
///
/// # Returns
///
/// A `SyncLoopExit` indicating whether the user wants to return to the menu or quit altogether.
pub async fn start_sync_loop(config: &Config) -> Result<SyncLoopExit, Box<dyn Error>> {
    // 1) Connect to OpenRGB
    let client = match OpenRGB::connect_to((&config.openrgb_host[..], config.openrgb_port)).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to OpenRGB server: {e}");
            return Ok(SyncLoopExit::ReturnToMenu);
        }
    };
    client.set_name("KeyBloom".to_string()).await?;

    // 2) Find the specified device
    let controller_count = client.get_controller_count().await?;
    let mut keyboard_id: Option<u32> = None;
    for i in 0..controller_count {
        if let Ok(ctrl) = client.get_controller(i).await {
            // You can refine this matching logic if needed
            if ctrl.name.contains(&config.device_name)
                || ctrl.name.to_lowercase().contains("keyboard")
            {
                keyboard_id = Some(i);
                break;
            }
        }
    }
    let kb_id = match keyboard_id {
        Some(id) => id,
        None => {
            eprintln!(
                "No device named '{}' found. Check your OpenRGB server.",
                config.device_name
            );
            return Ok(SyncLoopExit::ReturnToMenu);
        }
    };

    // Attempt to set custom mode (if supported)
    if let Err(e) = client.set_custom_mode(kb_id).await {
        eprintln!("Could not set custom mode on device: {e}");
    }

    // 3) Select monitor for screen capture
    let monitors = Monitor::all().map_err(|e| format!("xcap error: {e}"))?;
    let monitor = monitors
        .get(config.monitor_index)
        .unwrap_or_else(|| &monitors[0])
        .clone();

    println!(
        "\nSync started on monitor: {} ({}x{}), device: {}. Press 'm' to return to menu, 'q' to quit.\n",
        monitor.name(),
        monitor.width(),
        monitor.height(),
        config.device_name
    );

    let mut current_colors = vec![Color { r: 0, g: 0, b: 0 }; config.num_leds];
    let mut last_transition = Instant::now();
    let mut step_buffer = vec![Color { r: 0, g: 0, b: 0 }; config.num_leds];
    let color_threshold_sq = (config.color_change_threshold * 255.0).powi(2);
    let width = monitor.width() as usize;

    // Main capture-and-update loop
    loop {
        // Quick user input check
        if crossterm::event::poll(Duration::from_millis(1))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    match key.code {
                        crossterm::event::KeyCode::Char('m') => {
                            return Ok(SyncLoopExit::ReturnToMenu);
                        }
                        crossterm::event::KeyCode::Char('q') => {
                            return Ok(SyncLoopExit::Quit);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Capture screen
        let loop_start = Instant::now();
        let frame: RgbaImage = match monitor.capture_image() {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Capture error: {e}");
                sleep(Duration::from_millis(config.frame_delay_ms)).await;
                continue;
            }
        };

        let mut sums = frame
            .par_chunks_exact(width * 4)
            .map(|row_slice| {
                let mut row_sums = vec![(0u64, 0u64, 0u64, 0u64); config.num_leds];
                for x in 0..width {
                    let idx = x * 4;
                    let r = row_slice[idx] as u64;
                    let g = row_slice[idx + 1] as u64;
                    let b = row_slice[idx + 2] as u64;
                    let a = row_slice[idx + 3] as f32 / 255.0;

                    if a >= 0.1 {
                        // Which LED column does this pixel belong to?
                        let col_idx = (x * config.num_leds) / width;
                        let (rr, gg, bb, count) = &mut row_sums[col_idx];
                        *rr += r;
                        *gg += g;
                        *bb += b;
                        *count += 1;
                    }
                }
                row_sums
            })
            .reduce(
                || vec![(0u64, 0u64, 0u64, 0u64); config.num_leds],
                |mut acc, row_sums| {
                    for (i, (r, g, b, c)) in row_sums.into_iter().enumerate() {
                        let s = &mut acc[i];
                        s.0 += r;
                        s.1 += g;
                        s.2 += b;
                        s.3 += c;
                    }
                    acc
                },
            );

        // Convert sums to target colors, adjusting brightness & saturation
        let target_srgb: Vec<Srgb<f32>> = sums
            .par_iter()
            .map(|&(r_sum, g_sum, b_sum, count)| {
                if count == 0 {
                    Srgb::new(0.0, 0.0, 0.0)
                } else {
                    let count_f = count as f32;
                    let r_f = (r_sum as f32 / count_f) / 255.0;
                    let g_f = (g_sum as f32 / count_f) / 255.0;
                    let b_f = (b_sum as f32 / count_f) / 255.0;
                    let avg = Srgb::new(r_f, g_f, b_f);
                    let bright = increase_brightness(avg, config.brightness_factor);
                    adjust_saturation(bright, config.saturation_factor)
                }
            })
            .collect();

        let target_colors: Vec<Color> = target_srgb.iter().map(|&c| srgb_to_color(c)).collect();

        // Check if a significant color change occurred
        let significant_change = current_colors
            .iter()
            .zip(&target_colors)
            .any(|(curr, targ)| {
                let dr = targ.r as f32 - curr.r as f32;
                let dg = targ.g as f32 - curr.g as f32;
                let db = targ.b as f32 - curr.b as f32;
                let dist_sq = dr * dr + dg * dg + db * db;
                dist_sq > color_threshold_sq
            });

        let debounce_passed =
            last_transition.elapsed() >= Duration::from_millis(config.debounce_duration_ms);

        // If there's a big enough change and we've passed the debounce time, do a transition
        if significant_change && debounce_passed {
            if let Err(e) = smooth_transition(
                &client,
                kb_id,
                &mut current_colors,
                &target_colors,
                config,
                &mut step_buffer,
            )
            .await
            {
                eprintln!("Error updating keyboard LEDs: {e}");
            }
            last_transition = Instant::now();
        }

        // Honor frame delay
        let elapsed = loop_start.elapsed();
        if let Some(remaining) = Duration::from_millis(config.frame_delay_ms).checked_sub(elapsed) {
            sleep(remaining).await;
        }
    }
}

/// Smoothly transition `current` colors to `target` colors using HSV interpolation.
///
/// This function performs several incremental steps (specified in `config.transition_steps`)
/// and updates the keyboard LEDs at each step.
///
/// # Arguments
///
/// * `openrgb_client` - A reference to the connected OpenRGB client.
/// * `controller_id` - The numeric ID of the device being controlled.
/// * `current` - A mutable reference to the slice of current LED colors.
/// * `target` - A slice of target LED colors.
/// * `config` - The application configuration (`Config`).
/// * `step_buffer` - A mutable buffer used to store intermediate colors during each interpolation step.
///
/// # Returns
///
/// A `Result` indicating success or an `OpenRGBError`.
async fn smooth_transition(
    openrgb_client: &OpenRGB<tokio::net::TcpStream>,
    controller_id: u32,
    current: &mut [Color],
    target: &[Color],
    config: &Config,
    step_buffer: &mut [Color],
) -> Result<(), OpenRGBError> {
    if current.len() != target.len() || current.is_empty() {
        return Ok(());
    }
    let curr_srgb: Vec<Srgb<f32>> = current.iter().map(|&c| color_to_srgb(c)).collect();
    let targ_srgb: Vec<Srgb<f32>> = target.iter().map(|&c| color_to_srgb(c)).collect();

    for step in 1..=config.transition_steps {
        for (i, (cs, ts)) in curr_srgb.iter().zip(targ_srgb.iter()).enumerate() {
            let new_color = interpolate_color_hsv(*cs, *ts, step, config.transition_steps);
            step_buffer[i] = srgb_to_color(new_color);
        }
        openrgb_client.update_leds(controller_id, step_buffer.to_vec()).await?;
        current.copy_from_slice(step_buffer);
        tokio::time::sleep(Duration::from_millis(config.transition_delay_ms)).await;
    }
    Ok(())
}
