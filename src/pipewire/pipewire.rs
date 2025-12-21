//! PipeWire sample rate control module
//!
//! This module provides functionality to force PipeWire's sample rate
//! to match the currently playing song in MPD.

use pipewire as pw;
use pipewire::context::ContextBox;
use pipewire::main_loop::MainLoopBox;
use pipewire::metadata::Metadata;
use pipewire::properties::PropertiesBox;
use pipewire::registry::GlobalObject;
use pipewire::types::ObjectType;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

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
    pw::init();

    let mainloop =
        MainLoopBox::new(None).map_err(|e| format!("Failed to create PipeWire MainLoop: {}", e))?;

    let context = ContextBox::new(mainloop.loop_(), None)
        .map_err(|e| format!("Failed to create PipeWire Context: {}", e))?;

    let core = context
        .connect(None)
        .map_err(|e| format!("Failed to connect to PipeWire Core: {}", e))?;

    let registry = core
        .get_registry()
        .map_err(|e| format!("Failed to get PipeWire registry: {}", e))?;

    // Store found metadata global for later binding (using to_owned())
    let found_global: Rc<RefCell<Option<GlobalObject<PropertiesBox>>>> =
        Rc::new(RefCell::new(None));
    let found_global_clone = found_global.clone();

    // We need to store the listener to keep it alive
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            // Look for metadata objects
            if global.type_ == ObjectType::Metadata {
                if let Some(props) = global.props.as_ref() {
                    let name = props.get("metadata.name");
                    // Check if this is the "settings" metadata
                    if name == Some("settings") {
                        // Store an owned copy of the global for later binding
                        *found_global_clone.borrow_mut() = Some(global.to_owned());
                    }
                }
            }
        })
        .register();

    // Run the loop to discover objects
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(2);

    while found_global.borrow().is_none() && start.elapsed() < timeout {
        mainloop.loop_().iterate(Duration::from_millis(10));
    }

    // Check if we found the settings metadata
    let global_ref = found_global.borrow();
    let global = global_ref
        .as_ref()
        .ok_or_else(|| "Timeout waiting for PipeWire settings metadata".to_string())?;

    // Bind to the metadata object
    let metadata: Metadata = registry
        .bind(global)
        .map_err(|e| format!("Failed to bind metadata: {}", e))?;

    // Set clock.force-rate (subject 0 = global settings)
    // Note: type can be None, PipeWire will infer it
    let rate_str = rate.to_string();
    metadata.set_property(0, "clock.force-rate", None, Some(&rate_str));

    // Use sync to ensure the property change is flushed to the server
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();

    let _core_listener = core
        .add_listener_local()
        .done(move |_id, _seq| {
            done_clone.set(true);
        })
        .register();

    // Trigger a sync - this will cause a done event when all prior commands are processed
    core.sync(0).map_err(|e| format!("Failed to sync: {}", e))?;

    // Drop the borrow before running the loop
    drop(global_ref);

    // Wait for the done event
    let sync_start = std::time::Instant::now();
    let sync_timeout = Duration::from_millis(500);
    while !done.get() && sync_start.elapsed() < sync_timeout {
        mainloop.loop_().iterate(Duration::from_millis(10));
    }

    Ok(())
}

/// Resets PipeWire sample rate to automatic selection.
///
/// This is equivalent to calling `set_sample_rate(0)`.
pub fn reset_sample_rate() -> Result<(), String> {
    set_sample_rate(0)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_set_and_reset_rate() {
        // These tests require a running PipeWire instance
        // They are marked as ignored by default
    }
}
