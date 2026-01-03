//! PipeWire sample rate control module
//!
//! This module provides functionality to force PipeWire's sample rate
//! to match the currently playing song in MPD for bit-perfect playback.

use crate::app::logging::log_pipewire_operation;
use log::{debug, warn};
use pipewire::{
    context::ContextBox, main_loop::MainLoopBox, metadata::Metadata, properties::PropertiesBox,
    registry::GlobalObject, types::ObjectType,
};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;

/// The metadata name used by PipeWire for global settings
const SETTINGS_METADATA_NAME: &str = "settings";

/// Property key for forcing the clock rate
const CLOCK_FORCE_RATE_KEY: &str = "clock.force-rate";

/// Property key for allowed clock rates in settings metadata
/// Note: In metadata, it's "clock.allowed-rates" (not "default.clock.allowed-rates")
const CLOCK_ALLOWED_RATES_KEY: &str = "clock.allowed-rates";

/// Timeout for discovering PipeWire objects
const DISCOVERY_TIMEOUT: Duration = Duration::from_millis(50);

/// Timeout for sync operations
const SYNC_TIMEOUT: Duration = Duration::from_millis(25);

/// Iteration step for the main loop
const LOOP_ITERATION_STEP: Duration = Duration::from_millis(10);

/// Cache for supported sample rates - populated once on startup, valid for entire program lifetime
static SUPPORTED_RATES_CACHE: OnceLock<Vec<u32>> = OnceLock::new();

/// Forces PipeWire to use a specific sample rate via the settings metadata.
///
/// This function connects to PipeWire, finds the "settings" metadata object,
/// and sets the `clock.force-rate` property to the desired rate.
///
/// # Arguments
/// * `rate` - The sample rate to force. Use 0 to reset to automatic rate selection.
///
/// # Returns
/// * `Ok(())` if the rate was set successfully
/// * `Err(String)` with an error message if something went wrong
pub fn set_sample_rate(rate: u32) -> Result<(), String> {
    let result = set_sample_rate_inner(rate);

    // Log the operation result
    let operation = if rate == 0 {
        "reset_sample_rate"
    } else {
        "set_sample_rate"
    };

    match &result {
        Ok(()) => {
            let details = if rate == 0 {
                "automatic".to_string()
            } else {
                format!("{} Hz", rate)
            };
            log_pipewire_operation(operation, true, Some(&details));
        }
        Err(e) => {
            log_pipewire_operation(operation, false, Some(e));
        }
    }

    result
}

fn set_sample_rate_inner(rate: u32) -> Result<(), String> {
    // Initialize PipeWire library
    pipewire::init();

    // Create the main loop
    let mainloop =
        MainLoopBox::new(None).map_err(|e| format!("Failed to create PipeWire MainLoop: {e}"))?;

    // Create context from the main loop
    let context = ContextBox::new(mainloop.loop_(), None)
        .map_err(|e| format!("Failed to create PipeWire Context: {e}"))?;

    // Connect to the PipeWire server
    let core = context
        .connect(None)
        .map_err(|e| format!("Failed to connect to PipeWire Core: {e}"))?;

    // Get the registry to enumerate objects
    let registry = core
        .get_registry()
        .map_err(|e| format!("Failed to get PipeWire registry: {e}"))?;

    // Store found metadata global for later binding (using to_owned())
    let found_global: Rc<RefCell<Option<GlobalObject<PropertiesBox>>>> =
        Rc::new(RefCell::new(None));
    let found_global_clone = found_global.clone();

    // Register listener for global objects
    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            // Look for metadata objects with name "settings"
            if global.type_ == ObjectType::Metadata
                && let Some(props) = global.props.as_ref()
                && props.get("metadata.name") == Some(SETTINGS_METADATA_NAME)
            {
                // Store an owned copy of the global for later binding
                *found_global_clone.borrow_mut() = Some(global.to_owned());
            }
        })
        .register();

    // Run the loop to discover objects
    let start = std::time::Instant::now();
    while found_global.borrow().is_none() && start.elapsed() < DISCOVERY_TIMEOUT {
        mainloop.loop_().iterate(LOOP_ITERATION_STEP);
    }

    // Check if we found the settings metadata
    let global_ref = found_global.borrow();
    let global = global_ref
        .as_ref()
        .ok_or_else(|| "Timeout waiting for PipeWire settings metadata".to_string())?;

    // Bind to the metadata object
    let metadata: Metadata = registry
        .bind(global)
        .map_err(|e| format!("Failed to bind metadata: {e}"))?;

    // Set clock.force-rate (subject 0 = global settings)
    // When rate is 0, we delete the property (pass None) to reset to automatic
    // This triggers an immediate rate renegotiation, unlike setting to "0"
    if rate == 0 {
        metadata.set_property(0, CLOCK_FORCE_RATE_KEY, None, None);
        debug!("Reset PipeWire sample rate to automatic");
    } else {
        let rate_str = rate.to_string();
        metadata.set_property(0, CLOCK_FORCE_RATE_KEY, None, Some(&rate_str));
        debug!("Set PipeWire sample rate to {rate} Hz");
    }

    // Sync to ensure the property change is flushed to the server
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();

    let _core_listener = core
        .add_listener_local()
        .done(move |_id, _seq| {
            done_clone.set(true);
        })
        .register();

    // Trigger a sync - this will cause a done event when all prior commands are processed
    core.sync(0).map_err(|e| format!("Failed to sync: {e}"))?;

    // Drop the borrow before running the loop
    drop(global_ref);

    // Wait for the done event
    let sync_start = std::time::Instant::now();
    while !done.get() && sync_start.elapsed() < SYNC_TIMEOUT {
        mainloop.loop_().iterate(LOOP_ITERATION_STEP);
    }

    if !done.get() {
        warn!("PipeWire sync timed out, property may not be applied");
    }

    Ok(())
}

/// Resets PipeWire sample rate to automatic selection.
///
/// This clears the `clock.force-rate` property, allowing PipeWire
/// to automatically select the best sample rate.
#[cfg(target_os = "linux")]
pub fn reset_sample_rate() -> Result<(), String> {
    set_sample_rate(0)
}

/// Initialize the supported rates cache.
/// Should be called once when the program starts.
pub fn initialize_supported_rates() -> Result<Vec<u32>, String> {
    if let Some(rates) = SUPPORTED_RATES_CACHE.get() {
        return Ok(rates.clone());
    }

    let rates = get_supported_rates_inner()?;
    let _ = SUPPORTED_RATES_CACHE.set(rates.clone());
    Ok(rates)
}

/// Gets the list of supported sample rates from the cache.
///
/// This function only returns cached rates and never queries PipeWire.
/// Must call initialize_supported_rates() first.
///
/// # Returns
/// * `Some(Vec<u32>)` with the list of supported sample rates from cache
/// * `None` if cache hasn't been initialized
pub fn get_supported_rates() -> Option<Vec<u32>> {
    SUPPORTED_RATES_CACHE.get().cloned()
}

fn get_supported_rates_inner() -> Result<Vec<u32>, String> {
    // First try to read allowed-rates using pw-metadata command (most reliable)
    if let Some(rates) = get_allowed_rates_from_pw_metadata() {
        if !rates.is_empty() {
            debug!("Got allowed-rates from pw-metadata: {:?}", rates);
            return Ok(rates);
        }
    }

    // Fallback: try to read from PipeWire API directly
    if let Some(rates) = get_allowed_rates_from_api() {
        if !rates.is_empty() {
            debug!("Got allowed-rates from PipeWire API: {:?}", rates);
            return Ok(rates);
        }
    }

    // No allowed-rates configured means PipeWire allows any rate
    // Use common rates as reasonable defaults
    debug!("No allowed-rates found in PipeWire, using common rates");
    Ok(vec![
        44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000,
    ])
}

/// Read allowed-rates from pw-metadata command output
fn get_allowed_rates_from_pw_metadata() -> Option<Vec<u32>> {
    use std::process::Command;

    let output = Command::new("pw-metadata")
        .args(["-n", "settings"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for line like: update: id:0 key:'clock.allowed-rates' value:'[ 44100, 48000, 88200, 96000 ]' type:''
    for line in stdout.lines() {
        if line.contains("clock.allowed-rates") && line.contains("value:") {
            // Extract the value between value:' and ' type:
            if let Some(start) = line.find("value:'") {
                let value_start = start + 7; // len("value:'")
                if let Some(end) = line[value_start..].find("' type:") {
                    let value = &line[value_start..value_start + end];
                    let rates = parse_allowed_rates(value);
                    if !rates.is_empty() {
                        return Some(rates);
                    }
                }
            }
        }
    }

    None
}

/// Read allowed-rates from PipeWire API (fallback method)
fn get_allowed_rates_from_api() -> Option<Vec<u32>> {
    // Initialize PipeWire library
    pipewire::init();

    // Create the main loop
    let mainloop = MainLoopBox::new(None).ok()?;

    // Create context from the main loop
    let context = ContextBox::new(mainloop.loop_(), None).ok()?;

    // Connect to the PipeWire server
    let core = context.connect(None).ok()?;

    // Get the registry to enumerate objects
    let registry = core.get_registry().ok()?;

    // Store found metadata global for later binding
    let found_global: Rc<RefCell<Option<GlobalObject<PropertiesBox>>>> =
        Rc::new(RefCell::new(None));
    let found_global_clone = found_global.clone();

    // Store allowed rates from metadata
    let allowed_rates: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    let allowed_rates_clone = allowed_rates.clone();

    // Register listener for global objects
    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            // Look for metadata objects with name "settings"
            if global.type_ == ObjectType::Metadata
                && let Some(props) = global.props.as_ref()
                && props.get("metadata.name") == Some(SETTINGS_METADATA_NAME)
            {
                // Store an owned copy of the global for later binding
                *found_global_clone.borrow_mut() = Some(global.to_owned());
            }
        })
        .register();

    // Run the loop to discover objects
    let start = std::time::Instant::now();
    while found_global.borrow().is_none() && start.elapsed() < DISCOVERY_TIMEOUT {
        mainloop.loop_().iterate(LOOP_ITERATION_STEP);
    }

    // Check if we found the settings metadata
    let global_ref = found_global.borrow();
    if let Some(global) = global_ref.as_ref() {
        // Bind to the metadata object
        let metadata_result: Result<Metadata, _> = registry.bind(global);
        if let Ok(metadata) = metadata_result {
            let allowed_rates_listener = allowed_rates_clone.clone();

            // Register listener for metadata properties
            let _metadata_listener = metadata
                .add_listener_local()
                .property(move |_subject, key, _type, value| {
                    if let Some(key) = key
                        && key == CLOCK_ALLOWED_RATES_KEY
                        && let Some(value) = value
                    {
                        // Parse the allowed-rates array from JSON format: "[ 44100, 48000 ]"
                        let rates = parse_allowed_rates(value);
                        if !rates.is_empty() {
                            debug!("Found PipeWire allowed-rates: {:?}", rates);
                            *allowed_rates_listener.borrow_mut() = rates;
                        }
                    }
                    0 // Return 0 to continue iteration
                })
                .register();

            // Run the loop to receive metadata properties (use longer timeout)
            let prop_start = std::time::Instant::now();
            while allowed_rates_clone.borrow().is_empty()
                && prop_start.elapsed() < DISCOVERY_TIMEOUT
            {
                mainloop.loop_().iterate(LOOP_ITERATION_STEP);
            }
        }
    }

    // Drop the borrow before checking results
    drop(global_ref);

    // Return the rates if we found any
    let rates = allowed_rates.borrow().clone();
    if rates.is_empty() { None } else { Some(rates) }
}

/// Parse allowed-rates from PipeWire's JSON-like format
/// Examples: "[ 44100, 48000 ]" or "[ 44100 48000 ]"
fn parse_allowed_rates(value: &str) -> Vec<u32> {
    let mut rates = Vec::new();

    // Remove brackets and split by comma or whitespace
    let cleaned = value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();

    // Handle both comma-separated and space-separated formats
    for part in cleaned.split([',', ' ']) {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            if let Ok(rate) = trimmed.parse::<u32>() {
                if (8000..=384000).contains(&rate) && !rates.contains(&rate) {
                    rates.push(rate);
                }
            }
        }
    }

    rates.sort();
    rates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires a running PipeWire instance
    fn test_set_and_reset_rate() {
        // Set a sample rate
        let result = set_sample_rate(48000);
        assert!(result.is_ok(), "Failed to set sample rate: {:?}", result);

        // Reset to automatic
        let result = reset_sample_rate();
        assert!(result.is_ok(), "Failed to reset sample rate: {:?}", result);
    }

    #[test]
    #[ignore] // Requires a running PipeWire instance
    fn test_common_sample_rates() {
        let rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for rate in rates {
            let result = set_sample_rate(rate);
            assert!(
                result.is_ok(),
                "Failed to set sample rate {}: {:?}",
                rate,
                result
            );
        }

        // Clean up
        let _ = reset_sample_rate();
    }
}

/// Async wrapper for set_sample_rate that runs the blocking PipeWire call
/// on a separate thread to avoid blocking the tokio runtime.
pub async fn set_sample_rate_async(rate: u32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_sample_rate(rate))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}

/// Async wrapper for reset_sample_rate that runs the blocking PipeWire call
/// on a separate thread to avoid blocking the tokio runtime.
pub async fn reset_sample_rate_async() -> Result<(), String> {
    tokio::task::spawn_blocking(reset_sample_rate)
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}
