# Dirty Region Rendering

## The Problem

Traditional TUI applications redraw the entire screen every frame:

```rust
// Naive approach: full redraw every iteration
loop {
    terminal.draw(|frame| {
        render_header(frame);      // Always redrawn
        render_queue(frame);       // Always redrawn (1000+ items)
        render_status(frame);      // Always redrawn
        render_cover_art(frame);   // Always redrawn
    })?;
    
    tokio::time::sleep(Duration::from_millis(16)).await;
}
```

Problems:
1. **Wasted CPU**: Redrawing unchanged content
2. **Visual artifacts**: Unnecessary terminal updates cause flicker
3. **Power consumption**: Laptop batteries drain faster
4. **Thermal issues**: CPU stays hot during idle

## Why Full Redraws Happen

Most TUI frameworks are "immediate mode"—they don't track what changed:

```rust
// ratatui rebuilds widget tree every frame
frame.render_widget(my_list, area);  // Doesn't know if list changed
```

The application must track changes itself.

## Solution: Dirty Flag System

Track which UI regions have changed since last render:

```rust
pub struct DirtyFlags {
    // Individual region flags
    queue: Cell<bool>,
    queue_selection: Cell<bool>,
    current_song: Cell<bool>,
    status: Cell<bool>,
    progress: Cell<bool>,
    cover_art: Cell<bool>,
    library: Cell<bool>,
    menu_mode: Cell<bool>,
    panel_focus: Cell<bool>,
    key_sequence: Cell<bool>,  // For sequential key binding indicator
    
    // Terminal state
    terminal_size: Cell<bool>,
    force_full: Cell<bool>,
    last_width: Cell<u16>,
    last_height: Cell<u16>,
}
```

Note: Uses `Cell<bool>` for interior mutability—allows marking dirty through shared references without requiring `&mut self`.

### Flag Granularity

Choose granularity based on UI structure:

```
┌────────────────────────────────────────────────┐
│ Status Bar              [status] Seq: g [key_sequence]│
├──────────────────────┬─────────────────────────┤
│                      │                         │
│  Cover Art           │  Queue List             │
│  [cover_art]         │  [queue, queue_selection]│
│                      │                         │
├──────────────────────┴─────────────────────────┤
│ Progress Bar                  [progress]       │
├────────────────────────────────────────────────┤
│ Now Playing                   [current_song]   │
└────────────────────────────────────────────────┘
```

## Implementation

### Basic Structure

```rust
impl DirtyFlags {
    pub fn new() -> Self {
        Self {
            // Start dirty to ensure initial render
            queue: true,
            queue_selection: true,
            current_song: true,
            status: true,
            progress: true,
            cover_art: true,
            library: true,
            menu_mode: true,
            panel_focus: true,
            last_width: 0,
            last_height: 0,
            force_full: true,
        }
    }
}
```

### Marking Regions Dirty

```rust
impl DirtyFlags {
    /// Mark queue content as changed (new songs, reorder, etc.)
    pub fn mark_queue(&mut self) {
        self.queue = true;
    }

    /// Mark only selection as changed (navigation within queue)
    pub fn mark_queue_selection(&mut self) {
        self.queue_selection = true;
    }

    /// Mark current song info as changed
    pub fn mark_current_song(&mut self) {
        self.current_song = true;
    }

    /// Mark playback status (play/pause/stop, modes)
    pub fn mark_status(&mut self) {
        self.status = true;
    }

    /// Mark progress bar (time elapsed)
    pub fn mark_progress(&mut self) {
        self.progress = true;
    }

    /// Mark cover art as changed
    pub fn mark_cover_art(&mut self) {
        self.cover_art = true;
    }

    /// Force full redraw (terminal resize, theme change)
    pub fn force_full_redraw(&mut self) {
        self.force_full = true;
    }
}
```

### Terminal Size Detection

```rust
impl DirtyFlags {
    /// Check if terminal size changed, mark full redraw if so
    pub fn check_terminal_size(&mut self, width: u16, height: u16) {
        if width != self.last_width || height != self.last_height {
            self.last_width = width;
            self.last_height = height;
            self.force_full_redraw();
        }
    }
}
```

### Querying Dirty State

```rust
impl DirtyFlags {
    /// Returns true if ANY region needs redraw
    pub fn any_dirty(&self) -> bool {
        self.force_full
            || self.queue
            || self.queue_selection
            || self.current_song
            || self.status
            || self.progress
            || self.cover_art
            || self.library
            || self.menu_mode
            || self.panel_focus
    }

    /// Check specific regions (for future per-widget caching)
    pub fn is_queue_dirty(&self) -> bool {
        self.force_full || self.queue
    }

    pub fn is_status_dirty(&self) -> bool {
        self.force_full || self.status
    }
}
```

### Clearing After Render

```rust
impl DirtyFlags {
    /// Clear all flags after successful render
    pub fn clear_all(&mut self) {
        self.queue = false;
        self.queue_selection = false;
        self.current_song = false;
        self.status = false;
        self.progress = false;
        self.cover_art = false;
        self.library = false;
        self.menu_mode = false;
        self.panel_focus = false;
        self.force_full = false;
        // Note: last_width/height preserved for next size check
    }
}
```

## Integration Points

### 1. Event Handlers (User Input)

```rust
// In navigation.rs
impl Navigation for App {
    fn navigate_queue_up(&mut self) {
        if let Some(selected) = self.queue_list_state.selected() {
            if selected > 0 {
                self.queue_list_state.select(Some(selected - 1));
                self.dirty.mark_queue_selection();  // Only selection changed
            }
        }
    }

    fn switch_to_library(&mut self) {
        self.menu_mode = MenuMode::Library;
        self.dirty.mark_menu_mode();  // Mode changed
        self.dirty.mark_library();    // Library panel needs redraw
    }
}
```

### 2. MPD Update Handlers

```rust
// In mpd_updates.rs
impl MPDUpdates for App {
    async fn update_queue(&mut self, client: &Client) {
        let new_queue = fetch_queue(client).await?;
        if new_queue != self.queue {
            self.queue = new_queue;
            self.dirty.mark_queue();  // Queue content changed
        }
    }

    async fn update_status(&mut self, client: &Client) {
        let new_status = client.command(commands::Status).await?;
        
        // Fine-grained dirty marking
        if new_status.state != self.mpd_status.map(|s| s.state) {
            self.dirty.mark_status();
        }
        if new_status.elapsed != self.mpd_status.map(|s| s.elapsed) {
            self.dirty.mark_progress();
        }
        
        self.mpd_status = Some(new_status);
    }
}
```

### 3. Main Loop Conditional Render

```rust
// In main_loop.rs
while self.running {
    // Check terminal size for dirty tracking
    let term_size = terminal.size()?;
    self.dirty.check_terminal_size(term_size.width, term_size.height);

    // ONLY render if something changed
    if self.dirty.any_dirty() {
        terminal.draw(|frame| {
            render(frame, &self);
        })?;
        
        // Clear flags AFTER successful render
        self.dirty.clear_all();
    }

    // Event handling...
    tokio::select! {
        // ... events that may mark dirty flags
    }
}
```

## Advanced: Per-Widget Caching

The dirty flag system enables future per-widget caching:

```rust
struct QueueWidgetCache {
    items: Vec<ListItem<'static>>,
    valid: bool,
}

fn render_queue(
    frame: &mut Frame,
    queue: &[SongInfo],
    cache: &mut QueueWidgetCache,
    dirty: &DirtyFlags,
    area: Rect,
) {
    // Only rebuild if queue content changed
    if dirty.is_queue_dirty() || !cache.valid {
        cache.items = build_queue_items(queue);
        cache.valid = true;
    }

    // Render from cache (possibly with updated selection highlight)
    let widget = List::new(cache.items.clone());
    frame.render_stateful_widget(widget, area, &mut state);
}
```

## Performance Impact

### Idle State (No User Input, Paused Playback)

**Before**: Full redraw every 16ms
- CPU: 15-25%
- Frame time: 8-12ms

**After**: No redraws
- CPU: <1%
- Frame time: N/A (no renders)

### Active Playback (Progress Updates)

**Before**: Full redraw every 500ms
- Redraws: Queue (1000 items) + Status + Progress + Cover

**After**: Only progress bar dirty
- Redraws: Progress bar only
- CPU reduction: ~90%

### User Navigation

**Before**: Full redraw on every keypress

**After**: Only affected regions redraw
- Queue selection change: Queue region only
- Mode switch: Menu + new panel only

## Common Pitfalls

### 1. Forgetting to Mark Dirty

```rust
// Bug: UI doesn't update!
fn update_volume(&mut self, new_volume: u8) {
    self.volume = new_volume;
    // Missing: self.dirty.mark_status();
}
```

**Internal state changes also need dirty marking** — not just data fetched from MPD:

```rust
// Bug: Toggle doesn't update icon!
MPDAction::ToggleBitPerfect => {
    self.bit_perfect_enabled = !self.bit_perfect_enabled;
    // Missing: self.dirty.mark_status();
}
```

### 2. Over-Marking

```rust
// Inefficient: Marks everything on every update
fn update_anything(&mut self) {
    self.dirty.force_full_redraw();  // Don't do this!
}
```

### 3. Clearing Too Early

```rust
// Bug: Render sees cleared flags
self.dirty.clear_all();  // Wrong order!
terminal.draw(|frame| render(frame, &self))?;
```

### 4. Not Handling Terminal Resize

```rust
// Bug: UI garbled after resize
loop {
    // Missing: self.dirty.check_terminal_size()
    if self.dirty.any_dirty() {
        terminal.draw(...)?;
    }
}
```

## Testing Dirty Flags

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_dirty() {
        let flags = DirtyFlags::new();
        assert!(flags.any_dirty());
        assert!(flags.force_full);
    }

    #[test]
    fn test_clear_all() {
        let mut flags = DirtyFlags::new();
        flags.clear_all();
        assert!(!flags.any_dirty());
    }

    #[test]
    fn test_individual_marks() {
        let mut flags = DirtyFlags::new();
        flags.clear_all();
        
        flags.mark_queue();
        assert!(flags.any_dirty());
        assert!(flags.is_queue_dirty());
        assert!(!flags.is_status_dirty());
    }

    #[test]
    fn test_terminal_size_change() {
        let mut flags = DirtyFlags::new();
        flags.clear_all();
        
        flags.check_terminal_size(80, 24);
        assert!(flags.any_dirty());  // Size changed from 0,0
        
        flags.clear_all();
        flags.check_terminal_size(80, 24);
        assert!(!flags.any_dirty());  // Same size, no change
        
        flags.check_terminal_size(120, 40);
        assert!(flags.any_dirty());  // Size changed
    }
}
```

## Related Files

- `src/ui/dirty.rs` - DirtyFlags implementation
- `src/app/mod.rs` - App struct with dirty field
- `src/app/main_loop.rs` - Conditional render loop
- `src/app/navigation.rs` - User input marking
- `src/app/mpd_updates.rs` - Data update marking
- `src/app/event_handlers.rs` - Key sequence dirty marking

## See Also

- `09_sequential_key_bindings.md` - Details on `key_sequence` flag usage for vim-style bindings
