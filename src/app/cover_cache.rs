//! Cover Art Cache with LRU eviction and prefetching support.
//!
//! This module provides caching for album cover art to avoid repeated MPD fetches
//! when navigating between songs. It prefetches cover art for adjacent queue items
//! to provide instant cover art display when tracks change.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of cached cover art entries
const MAX_CACHE_ENTRIES: usize = 20;

/// Number of queue items to prefetch ahead/behind
const PREFETCH_AHEAD: usize = 3;
const PREFETCH_BEHIND: usize = 1;

/// Cached cover art data
#[derive(Debug, Clone)]
pub struct CachedCover {
    /// Raw image bytes (None means no cover art available)
    pub data: Option<Vec<u8>>,
}

/// Thread-safe cover art cache with LRU eviction
#[derive(Debug)]
pub struct CoverArtCache {
    /// Map from file path to cached cover data
    entries: HashMap<PathBuf, CachedCover>,
    /// LRU order (front = oldest, back = most recent)
    lru_order: VecDeque<PathBuf>,
    /// Paths currently being fetched (to avoid duplicate requests)
    pending: std::collections::HashSet<PathBuf>,
    /// Cache statistics
    hits: u64,
    misses: u64,
}

impl CoverArtCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_CACHE_ENTRIES),
            lru_order: VecDeque::with_capacity(MAX_CACHE_ENTRIES),
            pending: std::collections::HashSet::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get cached cover art for a file path
    pub fn get(&mut self, path: &PathBuf) -> Option<&CachedCover> {
        if self.entries.contains_key(path) {
            self.hits += 1;
            // Move to back of LRU (most recently used)
            self.lru_order.retain(|p| p != path);
            self.lru_order.push_back(path.clone());
            self.entries.get(path)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Check if a path is cached (without updating LRU or stats)
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.entries.contains_key(path)
    }

    /// Insert cover art into the cache
    pub fn insert(&mut self, path: PathBuf, data: Option<Vec<u8>>) {
        // Remove from pending
        self.pending.remove(&path);

        // If already cached, just update and refresh LRU
        if self.entries.contains_key(&path) {
            self.lru_order.retain(|p| p != &path);
            self.lru_order.push_back(path.clone());
            self.entries.insert(path, CachedCover { data });
            return;
        }

        // Evict oldest if at capacity
        while self.entries.len() >= MAX_CACHE_ENTRIES {
            if let Some(oldest) = self.lru_order.pop_front() {
                self.entries.remove(&oldest);
                log::debug!("Evicted cover art cache entry: {:?}", oldest);
            } else {
                break;
            }
        }

        // Insert new entry
        self.lru_order.push_back(path.clone());
        self.entries.insert(path, CachedCover { data });
    }

    /// Mark a path as currently being fetched
    pub fn mark_pending(&mut self, path: PathBuf) {
        self.pending.insert(path);
    }

    /// Check if a path is pending fetch
    pub fn is_pending(&self, path: &PathBuf) -> bool {
        self.pending.contains(path)
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> (u64, u64, usize) {
        (self.hits, self.misses, self.entries.len())
    }

    /// Log cache statistics
    pub fn log_stats(&self) {
        let total = self.hits + self.misses;
        if total > 0 {
            let hit_rate = (self.hits as f64 / total as f64) * 100.0;
            log::debug!(
                "Cover art cache: {} hits, {} misses ({:.1}% hit rate), {} entries",
                self.hits,
                self.misses,
                hit_rate,
                self.entries.len()
            );
        }
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
        self.pending.clear();
    }
}

impl Default for CoverArtCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared cache wrapped in Arc<RwLock> for async access
pub type SharedCoverCache = Arc<RwLock<CoverArtCache>>;

/// Create a new shared cover cache
pub fn new_shared_cache() -> SharedCoverCache {
    Arc::new(RwLock::new(CoverArtCache::new()))
}

/// Determine which queue items should be prefetched based on current position
pub fn get_prefetch_targets(
    queue: &[crate::song::SongInfo],
    current_index: Option<usize>,
) -> Vec<PathBuf> {
    let mut targets = Vec::new();

    let Some(current_idx) = current_index else {
        return targets;
    };

    // Prefetch ahead
    for i in 1..=PREFETCH_AHEAD {
        let idx = current_idx.saturating_add(i);
        if idx < queue.len() {
            targets.push(queue[idx].file_path.clone());
        }
    }

    // Prefetch behind (for going back)
    for i in 1..=PREFETCH_BEHIND {
        if let Some(idx) = current_idx.checked_sub(i) {
            targets.push(queue[idx].file_path.clone());
        }
    }

    targets
}

/// Find the current song's index in the queue
pub fn find_current_index(
    queue: &[crate::song::SongInfo],
    current_song: &Option<crate::song::SongInfo>,
) -> Option<usize> {
    let current = current_song.as_ref()?;
    queue.iter().position(|s| s.file_path == current.file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = CoverArtCache::new();
        let path = PathBuf::from("/music/song.mp3");

        cache.insert(path.clone(), Some(vec![1, 2, 3]));

        let cached = cache.get(&path);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = CoverArtCache::new();

        // Fill cache beyond capacity
        for i in 0..(MAX_CACHE_ENTRIES + 5) {
            let path = PathBuf::from(format!("/music/song{}.mp3", i));
            cache.insert(path, Some(vec![i as u8]));
        }

        // Should have evicted oldest entries
        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);

        // First entries should be evicted
        assert!(!cache.contains(&PathBuf::from("/music/song0.mp3")));
        assert!(!cache.contains(&PathBuf::from("/music/song4.mp3")));

        // Last entries should still be present
        let last_idx = MAX_CACHE_ENTRIES + 4;
        assert!(cache.contains(&PathBuf::from(format!("/music/song{}.mp3", last_idx))));
    }

    #[test]
    fn test_cache_none_data() {
        let mut cache = CoverArtCache::new();
        let path = PathBuf::from("/music/no_cover.mp3");

        // Should be able to cache "no cover" result
        cache.insert(path.clone(), None);

        let cached = cache.get(&path);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data, None);
    }

    #[test]
    fn test_pending_tracking() {
        let mut cache = CoverArtCache::new();
        let path = PathBuf::from("/music/song.mp3");

        assert!(!cache.is_pending(&path));
        cache.mark_pending(path.clone());
        assert!(cache.is_pending(&path));

        // Insert clears pending
        cache.insert(path.clone(), Some(vec![]));
        assert!(!cache.is_pending(&path));
    }
}
