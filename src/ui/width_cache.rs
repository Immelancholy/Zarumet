//! Unicode width caching module for optimized text rendering
//!
//! This module provides a caching layer for Unicode string width calculations,
//! which are computationally expensive due to the need to iterate over each
//! character and look up its display width.
//!
//! The cache uses:
//! - TTL-based cleanup (stale entries removed after 30 seconds)
//! - LRU-style eviction (max 1000 entries)
//! - Thread-local storage for ratatui's single-threaded context

use log::debug;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

/// Default time-to-live for cache entries (30 seconds)
const DEFAULT_TTL: Duration = Duration::from_secs(30);

/// Maximum number of entries before triggering cleanup
const MAX_ENTRIES: usize = 1000;

/// Entry in the width cache containing the calculated width and metadata
#[derive(Clone, Debug)]
struct CacheEntry {
    /// The calculated Unicode display width
    width: usize,
    /// When this entry was last accessed
    last_access: Instant,
    /// Number of times this entry has been accessed
    access_count: u32,
}

impl CacheEntry {
    fn new(width: usize) -> Self {
        Self {
            width,
            last_access: Instant::now(),
            access_count: 1,
        }
    }

    fn touch(&mut self) {
        self.last_access = Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    fn is_stale(&self, ttl: Duration) -> bool {
        self.last_access.elapsed() > ttl
    }
}

/// Cache for Unicode string width calculations
///
/// This cache stores the display width of strings to avoid repeated
/// Unicode width calculations during rendering. It uses a simple
/// HashMap with TTL-based cleanup and entry limits.
#[derive(Debug)]
pub struct WidthCache {
    /// The actual cache storage
    entries: HashMap<String, CacheEntry>,
    /// Time-to-live for cache entries
    ttl: Duration,
    /// Last time cleanup was performed
    last_cleanup: Instant,
    /// Performance metrics: cache hits
    hits: u64,
    /// Performance metrics: cache misses
    misses: u64,
}

impl Default for WidthCache {
    fn default() -> Self {
        Self::new()
    }
}

impl WidthCache {
    /// Create a new empty width cache with default settings
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(256),
            ttl: DEFAULT_TTL,
            last_cleanup: Instant::now(),
            hits: 0,
            misses: 0,
        }
    }

    /// Create a new width cache with custom TTL
    #[allow(dead_code)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: HashMap::with_capacity(256),
            ttl,
            last_cleanup: Instant::now(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get the cached width of a string, calculating and caching if not present
    pub fn get_width(&mut self, s: &str) -> usize {
        // Trigger cleanup if needed
        self.maybe_cleanup();

        if let Some(entry) = self.entries.get_mut(s) {
            entry.touch();
            self.hits += 1;
            return entry.width;
        }

        // Cache miss - calculate width
        self.misses += 1;
        let width = s.width();
        self.entries.insert(s.to_string(), CacheEntry::new(width));

        width
    }

    /// Peek at cached width without updating access time or triggering cache miss
    /// Returns None if the string is not in cache
    pub fn peek_width(&self, s: &str) -> Option<usize> {
        self.entries.get(s).map(|entry| entry.width)
    }

    /// Check if a string is in the cache
    #[allow(dead_code)]
    pub fn contains(&self, s: &str) -> bool {
        self.entries.contains_key(s)
    }

    /// Check if the cache is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries in the cache
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get the cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// Get total number of cache accesses
    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }

    /// Get cache hits count
    #[allow(dead_code)]
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get cache misses count
    #[allow(dead_code)]
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Log cache statistics for debugging
    pub fn log_stats(&self) {
        debug!(
            "WidthCache stats: entries={}, hits={}, misses={}, hit_rate={:.1}%",
            self.entries.len(),
            self.hits,
            self.misses,
            self.hit_rate() * 100.0
        );
    }

    /// Clear all entries from the cache
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Perform cleanup if conditions are met (time elapsed or too many entries)
    fn maybe_cleanup(&mut self) {
        // Check if cleanup is needed
        let should_cleanup =
            self.entries.len() > MAX_ENTRIES || self.last_cleanup.elapsed() > self.ttl;

        if !should_cleanup {
            return;
        }

        self.cleanup();
    }

    /// Force cleanup of stale entries
    fn cleanup(&mut self) {
        let ttl = self.ttl;

        // Remove stale entries
        self.entries.retain(|_, entry| !entry.is_stale(ttl));

        // If still over limit, remove least accessed entries
        if self.entries.len() > MAX_ENTRIES {
            // Collect entries with their access counts
            let mut entries_by_access: Vec<_> = self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.access_count))
                .collect();

            // Sort by access count (ascending)
            entries_by_access.sort_by_key(|(_, count)| *count);

            // Remove the least accessed entries until we're under the limit
            let to_remove = self.entries.len() - (MAX_ENTRIES * 3 / 4); // Keep 75% of max
            for (key, _) in entries_by_access.into_iter().take(to_remove) {
                self.entries.remove(&key);
            }
        }

        self.last_cleanup = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_caching() {
        let mut cache = WidthCache::new();

        // First access should be a miss
        let width = cache.get_width("hello");
        assert_eq!(width, 5);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 0);

        // Second access should be a hit
        let width = cache.get_width("hello");
        assert_eq!(width, 5);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hits(), 1);
    }

    #[test]
    fn test_unicode_width() {
        let mut cache = WidthCache::new();

        // ASCII
        assert_eq!(cache.get_width("hello"), 5);

        // CJK characters (typically 2 cells wide each)
        assert_eq!(cache.get_width("中文"), 4);

        // Mixed content
        let mixed = "Hello 世界";
        let width = cache.get_width(mixed);
        assert_eq!(width, 10); // 6 ASCII + 4 CJK
    }

    #[test]
    fn test_peek_width() {
        let mut cache = WidthCache::new();

        // Peek on non-existent entry
        assert_eq!(cache.peek_width("test"), None);

        // Add entry
        cache.get_width("test");

        // Peek should now return the width
        assert_eq!(cache.peek_width("test"), Some(4));
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = WidthCache::new();

        assert_eq!(cache.hit_rate(), 0.0); // No accesses yet

        cache.get_width("a"); // miss
        cache.get_width("a"); // hit
        cache.get_width("a"); // hit

        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_empty_string() {
        let mut cache = WidthCache::new();
        assert_eq!(cache.get_width(""), 0);
    }

    #[test]
    #[ignore] // Performance benchmarks are non-deterministic; run with --ignored
    fn bench_uncached_vs_cached() {
        use std::time::Instant;

        // Test data with various Unicode characters
        let test_strings = vec![
            "Simple ASCII text",
            "Café with accents",
            "中文 Chinese characters",
            "🦀 Rust emoji 🚀",
            "Mixed: Hello 世界 🌍",
            "Very long string that needs truncation and has Unicode characters like café and 中文",
            "Artist Name - Album Title (2023)",
            "01. Song Title with Special Characters: ♫ ♪ ♬",
        ];

        // Benchmark uncached version
        let start = Instant::now();
        let mut total_width_uncached = 0;
        for _ in 0..1000 {
            for s in &test_strings {
                total_width_uncached += s.width();
            }
        }
        let uncached_duration = start.elapsed();

        // Benchmark cached version
        let mut cache = WidthCache::new();
        let start = Instant::now();
        let mut total_width_cached = 0;
        for _ in 0..1000 {
            for s in &test_strings {
                total_width_cached += cache.get_width(s);
            }
        }
        let cached_duration = start.elapsed();

        // Verify results are the same
        assert_eq!(total_width_uncached, total_width_cached);

        // Report performance improvement
        let improvement = uncached_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64;
        let hit_rate = cache.hit_rate();

        println!("\nPerformance Benchmark Results:");
        println!("  Uncached: {:?}", uncached_duration);
        println!("  Cached:   {:?}", cached_duration);
        println!("  Speedup:  {:.2}x", improvement);
        println!("  Hit rate: {:.1}%", hit_rate * 100.0);
        println!("  Cache entries: {}", cache.len());

        // Cache should have all test strings after first iteration
        assert!(
            hit_rate > 0.99,
            "Hit rate should be very high after warm-up"
        );
        assert!(improvement > 1.5, "Should see at least 1.5x improvement");
    }

    #[test]
    #[ignore] // Performance benchmarks are non-deterministic; run with --ignored
    fn bench_truncation_performance() {
        use crate::ui::utils::{truncate_by_width, truncate_by_width_cached};
        use std::time::Instant;

        let test_strings = vec![
            "This is a very long string that will need truncation",
            "Short",
            "Mixed Unicode: Hello 世界 🌍 with emoji",
            "Artist Name - Very Long Album Title (Special Edition)",
        ];

        let max_width = 20;
        let iterations = 500;

        // Benchmark uncached truncation
        let start = Instant::now();
        for _ in 0..iterations {
            for s in &test_strings {
                let _ = truncate_by_width(s, max_width);
            }
        }
        let uncached_duration = start.elapsed();

        // Benchmark cached truncation
        let mut cache = WidthCache::new();
        let start = Instant::now();
        for _ in 0..iterations {
            for s in &test_strings {
                let _ = truncate_by_width_cached(&mut cache, s, max_width);
            }
        }
        let cached_duration = start.elapsed();

        let improvement = uncached_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64;

        println!("\nTruncation Benchmark Results:");
        println!("  Uncached: {:?}", uncached_duration);
        println!("  Cached:   {:?}", cached_duration);
        println!("  Speedup:  {:.2}x", improvement);

        // Note: improvement may be modest for truncation since we still iterate chars
        // The main benefit is avoiding repeated full-width calculations for fits-check
    }

    #[test]
    #[ignore] // Performance benchmarks are non-deterministic; run with --ignored
    fn stress_test_large_library() {
        use crate::ui::utils::left_align_cached;
        use std::time::Instant;

        // Simulate a large music library with diverse Unicode content
        let artists = vec![
            "The Beatles",
            "Miles Davis",
            "Johann Sebastian Bach",
            "Björk",
            "坂本龍一", // Ryuichi Sakamoto
            "Sigur Rós",
            "Ólafur Arnalds",
            "Café Tacvba",
            "Мумий Тролль",    // Russian
            "久石譲",          // Joe Hisaishi
            "ישראל קטורזה",    // Hebrew
            "محمد عبد الوهاب", // Arabic
            "🎵 Electronic Artist 🎶",
        ];

        let albums = vec![
            "Abbey Road",
            "Kind of Blue",
            "The Well-Tempered Clavier",
            "Homogenic",
            "千と千尋の神隠し",
            "Takk...",
            "Island Songs",
            "Re",
            "Точка невозврата",
            "菊次郎の夏",
            "Greatest Hits Vol. 1",
            "🌍 World Music Collection 🌎",
        ];

        let song_titles = vec![
            "Come Together",
            "So What",
            "Prelude in C Major",
            "Jóga",
            "One Summer's Day (あの夏へ)",
            "Hoppípolla",
            "Near Light",
            "La Ingrata",
            "Владивосток 2000",
            "Summer (菊次郎の夏)",
            "Track #01 - Introduction",
            "🎼 Symphony No. 5 🎻",
        ];

        // Generate a large dataset simulating 5000 songs
        let mut test_data: Vec<String> = Vec::with_capacity(5000);
        for i in 0..5000 {
            let artist = artists[i % artists.len()];
            let album = albums[i % albums.len()];
            let title = song_titles[i % song_titles.len()];
            test_data.push(format!("{} - {} - {}", artist, album, title));
        }

        // Simulate rendering a queue/list view (common operation)
        let field_width = 40;
        let visible_items = 50; // Typical visible items in a terminal
        let frame_count = 100; // Simulate 100 frame renders

        // Benchmark without cache (simulating old behavior with direct width calculation)
        let start = Instant::now();
        for _frame in 0..frame_count {
            // Simulate scrolling through different parts of the list
            for i in 0..visible_items {
                let idx = (i * 100) % test_data.len();
                let s = &test_data[idx];
                // Simulate left_align without cache
                let display_width = s.width();
                let _result = if display_width >= field_width {
                    crate::ui::utils::truncate_by_width(s, field_width)
                } else {
                    format!("{}{}", s, " ".repeat(field_width - display_width))
                };
            }
        }
        let uncached_duration = start.elapsed();

        // Benchmark with cache
        let mut cache = WidthCache::new();
        let start = Instant::now();
        for _frame in 0..frame_count {
            // Same scrolling pattern
            for i in 0..visible_items {
                let idx = (i * 100) % test_data.len();
                let _result = left_align_cached(&mut cache, &test_data[idx], field_width);
            }
        }
        let cached_duration = start.elapsed();

        let improvement = uncached_duration.as_nanos() as f64 / cached_duration.as_nanos() as f64;
        let hit_rate = cache.hit_rate();

        println!("\nLarge Library Stress Test Results:");
        println!(
            "  Simulated: 5000 songs, {} visible items, {} frames",
            visible_items, frame_count
        );
        println!("  Uncached: {:?}", uncached_duration);
        println!("  Cached:   {:?}", cached_duration);
        println!("  Speedup:  {:.2}x", improvement);
        println!("  Hit rate: {:.1}%", hit_rate * 100.0);
        println!("  Cache entries: {}", cache.len());
        println!("  Total accesses: {}", cache.total_accesses());

        // With repeated access patterns, we should see good cache hit rates
        // Cache entries should be limited (not all 5000 songs cached, just viewed ones)
        assert!(
            cache.len() <= 100,
            "Cache should only contain recently viewed items, not entire library"
        );
    }
}
