//! Dirty region tracking for optimized rendering.
//!
//! This module provides a system to track which UI regions have changed
//! and need to be rebuilt. This allows the renderer to skip expensive
//! widget construction for unchanged regions.

use std::cell::Cell;

/// Tracks which UI regions need to be rebuilt.
///
/// Each flag indicates whether the corresponding UI region's data has changed
/// since the last render. When a flag is set, the renderer should rebuild
/// that widget. After rendering, all flags are cleared.
#[derive(Debug, Default)]
pub struct DirtyFlags {
    /// Queue list has changed (items added/removed/reordered)
    queue: Cell<bool>,
    /// Queue selection has changed (different item selected)
    queue_selection: Cell<bool>,
    /// Current song has changed
    current_song: Cell<bool>,
    /// MPD status has changed (volume, play state, options)
    status: Cell<bool>,
    /// Progress/elapsed time has changed
    progress: Cell<bool>,
    /// Cover art has changed
    cover_art: Cell<bool>,
    /// Library data has changed
    library: Cell<bool>,
    /// Menu mode has changed
    menu_mode: Cell<bool>,
    /// Panel focus has changed
    panel_focus: Cell<bool>,
    /// Terminal size has changed (forces full redraw)
    terminal_size: Cell<bool>,
    /// Force full redraw (initial render, etc.)
    force_full: Cell<bool>,
    /// Key sequence indicator needs update
    key_sequence: Cell<bool>,
    /// Last known terminal width
    last_width: Cell<u16>,
    /// Last known terminal height
    last_height: Cell<u16>,
}

#[allow(dead_code)] // Methods reserved for future per-widget dirty checking
impl DirtyFlags {
    /// Create new dirty flags with all regions marked dirty (forces initial render)
    pub fn new() -> Self {
        Self {
            queue: Cell::new(true),
            queue_selection: Cell::new(true),
            current_song: Cell::new(true),
            status: Cell::new(true),
            progress: Cell::new(true),
            cover_art: Cell::new(true),
            library: Cell::new(true),
            menu_mode: Cell::new(true),
            panel_focus: Cell::new(true),
            terminal_size: Cell::new(true),
            force_full: Cell::new(true),
            key_sequence: Cell::new(false),
            last_width: Cell::new(0),
            last_height: Cell::new(0),
        }
    }

    // ----- Mark methods (set dirty) -----

    /// Mark queue as dirty (items changed)
    #[inline]
    pub fn mark_queue(&self) {
        self.queue.set(true);
    }

    /// Mark queue selection as dirty (selection changed)
    #[inline]
    pub fn mark_queue_selection(&self) {
        self.queue_selection.set(true);
    }

    /// Mark current song as dirty
    #[inline]
    pub fn mark_current_song(&self) {
        self.current_song.set(true);
    }

    /// Mark status as dirty (volume, play state, options changed)
    #[inline]
    pub fn mark_status(&self) {
        self.status.set(true);
    }

    /// Mark progress as dirty (elapsed time changed)
    #[inline]
    pub fn mark_progress(&self) {
        self.progress.set(true);
    }

    /// Mark cover art as dirty
    #[inline]
    pub fn mark_cover_art(&self) {
        self.cover_art.set(true);
    }

    /// Mark library as dirty
    #[inline]
    pub fn mark_library(&self) {
        self.library.set(true);
    }

    /// Mark menu mode as dirty
    #[inline]
    pub fn mark_menu_mode(&self) {
        self.menu_mode.set(true);
    }

    /// Mark panel focus as dirty
    #[inline]
    pub fn mark_panel_focus(&self) {
        self.panel_focus.set(true);
    }

    /// Force a full redraw of all regions
    #[inline]
    pub fn mark_full_redraw(&self) {
        self.force_full.set(true);
    }

    /// Mark key sequence indicator as dirty
    #[inline]
    pub fn mark_key_sequence(&self) {
        self.key_sequence.set(true);
    }

    /// Check and update terminal size, marking dirty if changed
    #[inline]
    pub fn check_terminal_size(&self, width: u16, height: u16) {
        if width != self.last_width.get() || height != self.last_height.get() {
            self.terminal_size.set(true);
            self.last_width.set(width);
            self.last_height.set(height);
        }
    }

    // ----- Query methods (check if dirty) -----

    /// Check if queue needs redraw
    #[inline]
    pub fn is_queue_dirty(&self) -> bool {
        self.force_full.get() || self.terminal_size.get() || self.queue.get()
    }

    /// Check if queue selection needs redraw
    #[inline]
    pub fn is_queue_selection_dirty(&self) -> bool {
        self.force_full.get()
            || self.terminal_size.get()
            || self.queue.get()
            || self.queue_selection.get()
    }

    /// Check if current song info needs redraw
    #[inline]
    pub fn is_current_song_dirty(&self) -> bool {
        self.force_full.get() || self.terminal_size.get() || self.current_song.get()
    }

    /// Check if status/top box needs redraw
    #[inline]
    pub fn is_status_dirty(&self) -> bool {
        self.force_full.get()
            || self.terminal_size.get()
            || self.status.get()
            || self.menu_mode.get()
    }

    /// Check if progress bar needs redraw
    #[inline]
    pub fn is_progress_dirty(&self) -> bool {
        self.force_full.get()
            || self.terminal_size.get()
            || self.progress.get()
            || self.status.get()
    }

    /// Check if cover art needs redraw
    #[inline]
    pub fn is_cover_art_dirty(&self) -> bool {
        self.force_full.get() || self.terminal_size.get() || self.cover_art.get()
    }

    /// Check if library panels need redraw
    #[inline]
    pub fn is_library_dirty(&self) -> bool {
        self.force_full.get()
            || self.terminal_size.get()
            || self.library.get()
            || self.panel_focus.get()
    }

    /// Check if any region is dirty (needs render)
    #[inline]
    pub fn any_dirty(&self) -> bool {
        self.force_full.get()
            || self.terminal_size.get()
            || self.queue.get()
            || self.queue_selection.get()
            || self.current_song.get()
            || self.status.get()
            || self.progress.get()
            || self.cover_art.get()
            || self.library.get()
            || self.menu_mode.get()
            || self.panel_focus.get()
            || self.key_sequence.get()
    }

    /// Check if a full redraw is needed (terminal resize, mode change, etc.)
    #[inline]
    pub fn needs_full_redraw(&self) -> bool {
        self.force_full.get() || self.terminal_size.get() || self.menu_mode.get()
    }

    // ----- Clear methods (after render) -----

    /// Clear all dirty flags after render
    #[inline]
    pub fn clear_all(&self) {
        self.queue.set(false);
        self.queue_selection.set(false);
        self.current_song.set(false);
        self.status.set(false);
        self.progress.set(false);
        self.cover_art.set(false);
        self.library.set(false);
        self.menu_mode.set(false);
        self.panel_focus.set(false);
        self.terminal_size.set(false);
        self.force_full.set(false);
        self.key_sequence.set(false);
    }

    /// Clear only progress dirty flag (for high-frequency progress updates)
    #[inline]
    pub fn clear_progress(&self) {
        self.progress.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_all_dirty() {
        let flags = DirtyFlags::new();
        assert!(flags.any_dirty());
        assert!(flags.needs_full_redraw());
        assert!(flags.is_queue_dirty());
        assert!(flags.is_progress_dirty());
    }

    #[test]
    fn test_clear_all() {
        let flags = DirtyFlags::new();
        flags.clear_all();
        assert!(!flags.any_dirty());
        assert!(!flags.needs_full_redraw());
    }

    #[test]
    fn test_individual_marks() {
        let flags = DirtyFlags::new();
        flags.clear_all();

        flags.mark_queue();
        assert!(flags.is_queue_dirty());
        assert!(!flags.is_progress_dirty());

        flags.clear_all();
        flags.mark_progress();
        assert!(flags.is_progress_dirty());
        assert!(!flags.is_queue_dirty());
    }

    #[test]
    fn test_terminal_size_change() {
        let flags = DirtyFlags::new();
        flags.clear_all();

        // Same size - no dirty
        flags.check_terminal_size(80, 24);
        flags.clear_all();
        flags.check_terminal_size(80, 24);
        assert!(!flags.needs_full_redraw());

        // Size changed - dirty
        flags.check_terminal_size(100, 30);
        assert!(flags.needs_full_redraw());
    }

    #[test]
    fn test_force_full_affects_all() {
        let flags = DirtyFlags::new();
        flags.clear_all();

        flags.mark_full_redraw();
        assert!(flags.is_queue_dirty());
        assert!(flags.is_progress_dirty());
        assert!(flags.is_status_dirty());
        assert!(flags.is_cover_art_dirty());
    }
}
