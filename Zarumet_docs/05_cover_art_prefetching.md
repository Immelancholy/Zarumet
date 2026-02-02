# Cover Art Prefetching & LRU Cache

## The Problem

Loading album cover art involves:

1. **Network I/O**: Fetching from MPD server
2. **Decoding**: Parsing JPEG/PNG/WebP
3. **Resizing**: Fitting to terminal dimensions
4. **Protocol conversion**: Converting to terminal graphics protocol

This takes 100-500ms per image. Without caching:
- User skips tracks rapidly → UI freezes
- Same album's songs → redundant fetches
- Back/forward navigation → re-fetch previously loaded art

## Solution: LRU Cache with Prefetching

```
                    ┌─────────────────────────────┐
                    │       Cover Art Cache       │
                    │  ┌─────┬─────┬─────┬─────┐ │
                    │  │ A   │ B   │ C   │ D   │ │ ← LRU order
                    │  └─────┴─────┴─────┴─────┘ │
                    └─────────────────────────────┘
                              ▲
                              │
    ┌─────────────────────────┼─────────────────────────┐
    │                         │                         │
┌───┴───┐                ┌────┴────┐              ┌─────┴─────┐
│Prefetch│               │ Current │              │ Prefetch  │
│ Prev  │                │  Song   │              │   Next    │
└───────┘                └─────────┘              └───────────┘
```

## LRU Cache Implementation

### Core Structure

```rust
use std::path::PathBuf;
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct CoverArtEntry {
    pub data: Option<Vec<u8>>,  // None = no cover art exists
}

pub struct CoverCache {
    cache: LruCache<PathBuf, CoverArtEntry>,
    pending: std::collections::HashSet<PathBuf>,
    hits: u64,
    misses: u64,
}

impl CoverCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            pending: std::collections::HashSet::new(),
            hits: 0,
            misses: 0,
        }
    }
}
```

### Why Store `Option<Vec<u8>>`?

```rust
pub struct CoverArtEntry {
    pub data: Option<Vec<u8>>,  // None means "no cover art"
}
```

This caches **negative results**:
- `Some(vec![...])` = Cover art exists, here's the data
- `None` = We checked, no cover art for this file

Without negative caching:
```rust
// Bug: Re-fetches every time for songs without cover art
if !cache.contains(&path) {
    let art = fetch_cover_art(&path).await;  // Returns None
    // Not cached! Next render triggers another fetch
}
```

## Preventing Duplicate Fetches

### The Problem

```rust
// User rapidly skips 5 tracks to same album
// Without pending tracking:
fetch_cover_art("album/song1.flac");  // Started
fetch_cover_art("album/song2.flac");  // Started (same album!)
fetch_cover_art("album/song3.flac");  // Started (same album!)
// 3 redundant network requests
```

### Solution: Pending Set

```rust
impl CoverCache {
    /// Check if a fetch is already in progress
    pub fn is_pending(&self, path: &PathBuf) -> bool {
        self.pending.contains(path)
    }

    /// Mark a path as being fetched
    pub fn mark_pending(&mut self, path: PathBuf) {
        self.pending.insert(path);
    }

    /// Store result and clear pending status
    pub fn insert(&mut self, path: PathBuf, data: Option<Vec<u8>>) {
        self.pending.remove(&path);
        self.cache.put(path, CoverArtEntry { data });
    }
}
```

### Usage Pattern

```rust
async fn load_cover_art(path: PathBuf, cache: SharedCoverCache) {
    // Check cache and pending atomically
    {
        let mut guard = cache.write().await;
        
        if guard.contains(&path) {
            return;  // Already cached
        }
        
        if guard.is_pending(&path) {
            return;  // Already being fetched
        }
        
        guard.mark_pending(path.clone());
    }  // Release lock before I/O
    
    // Fetch (potentially slow)
    let data = client.album_art(&path).await;
    
    // Store result
    {
        let mut guard = cache.write().await;
        guard.insert(path, data);
    }
}
```

## Prefetching Strategy

### Predict What's Needed Next

```rust
/// Get paths to prefetch based on current queue position
pub fn get_prefetch_targets(
    queue: &[SongInfo],
    current_index: Option<usize>,
) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    
    if let Some(idx) = current_index {
        // Prefetch next 2 songs
        for i in 1..=2 {
            if let Some(song) = queue.get(idx + i) {
                targets.push(song.file_path.clone());
            }
        }
        
        // Prefetch previous song (for back navigation)
        if idx > 0 {
            if let Some(song) = queue.get(idx - 1) {
                targets.push(song.file_path.clone());
            }
        }
    }
    
    targets
}
```

### Trigger Prefetch on Song Change

```rust
fn on_song_change(&mut self, new_song: &SongInfo) {
    // Load current song's cover art
    spawn_cover_loader(new_song.file_path.clone());
    
    // Prefetch adjacent songs
    let current_idx = find_queue_index(&self.queue, new_song);
    let targets = get_prefetch_targets(&self.queue, current_idx);
    
    for path in targets {
        spawn_prefetch_loader(path, self.cover_cache.clone());
    }
}
```

### Prefetch vs. Primary Load

```rust
// Primary load: Send result to UI
fn spawn_cover_loader(path: PathBuf, tx: Sender<CoverArtMessage>) {
    tokio::spawn(async move {
        let data = fetch_cover_art(&path).await;
        let _ = tx.send(CoverArtMessage::Loaded(data, path)).await;
    });
}

// Prefetch: Just populate cache, no UI update
fn spawn_prefetch_loader(path: PathBuf, cache: SharedCoverCache) {
    tokio::spawn(async move {
        // Skip if already cached or pending
        {
            let guard = cache.read().await;
            if guard.contains(&path) || guard.is_pending(&path) {
                return;
            }
        }
        
        let data = fetch_cover_art(&path).await;
        
        {
            let mut guard = cache.write().await;
            guard.insert(path.clone(), data);
            log::debug!("Prefetched cover art: {:?}", path);
        }
    });
}
```

## Shared State with Arc<RwLock>

Since cover art loading happens on background tasks:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedCoverCache = Arc<RwLock<CoverCache>>;

pub fn new_shared_cache() -> SharedCoverCache {
    Arc::new(RwLock::new(CoverCache::new(50)))  // 50 entries
}
```

### Why RwLock Instead of Mutex?

```rust
// Multiple readers (cache checks) can proceed in parallel
let guard = cache.read().await;
if guard.contains(&path) { ... }

// Only writes need exclusive access
let mut guard = cache.write().await;
guard.insert(path, data);
```

Cache checks happen frequently; inserts are rare. RwLock optimizes for this pattern.

### Why tokio::sync::RwLock?

```rust
// std::sync::RwLock - Cannot hold across .await
let guard = std_rwlock.read().unwrap();
do_async_work().await;  // DEADLOCK RISK!

// tokio::sync::RwLock - Designed for async
let guard = tokio_rwlock.read().await;
do_async_work().await;  // Safe (but avoid if possible)
```

## Memory Management

### Cache Size Considerations

```rust
// Each cover art entry:
// - PathBuf key: ~100 bytes
// - Vec<u8> data: ~50KB-500KB (typical JPEG)
// - LRU node overhead: ~48 bytes

// 50 entries × ~300KB average = ~15MB max
pub fn new_shared_cache() -> SharedCoverCache {
    Arc::new(RwLock::new(CoverCache::new(50)))
}
```

### LRU Eviction

```rust
impl CoverCache {
    pub fn insert(&mut self, path: PathBuf, data: Option<Vec<u8>>) {
        self.pending.remove(&path);
        
        // LruCache::put automatically evicts oldest if at capacity
        self.cache.put(path, CoverArtEntry { data });
    }
}
```

### Memory Pressure Handling

```rust
impl CoverCache {
    /// Manually evict entries under memory pressure
    pub fn shrink(&mut self, target_size: usize) {
        while self.cache.len() > target_size {
            self.cache.pop_lru();
        }
    }
    
    /// Estimate current memory usage
    pub fn memory_usage(&self) -> usize {
        self.cache.iter()
            .map(|(path, entry)| {
                path.as_os_str().len()
                    + entry.data.as_ref().map(|d| d.len()).unwrap_or(0)
            })
            .sum()
    }
}
```

## Integration with UI

### Channel-Based Communication

```rust
enum CoverArtMessage {
    Loaded(Option<Vec<u8>>, PathBuf),
}

// In main loop
select! {
    Some(msg) = cover_rx.recv() => {
        match msg {
            CoverArtMessage::Loaded(data, path) => {
                // Only update if this is still the current song
                if current_song_path == Some(&path) {
                    protocol.image = decode_image(data);
                    dirty.mark_cover_art();
                }
            }
        }
    }
}
```

### Avoiding Stale Updates

```rust
// User rapidly changes songs: A → B → C
// Cover art loads complete: A (slow), B (medium), C (fast)

// Bad: Display A's cover art for song C
if let CoverArtMessage::Loaded(data, _path) = msg {
    protocol.image = decode_image(data);  // Wrong!
}

// Good: Verify path matches current song
if let CoverArtMessage::Loaded(data, path) = msg {
    if current_song_path.as_ref() == Some(&path) {
        protocol.image = decode_image(data);
    }
}
```

## Performance Metrics

```rust
impl CoverCache {
    pub fn log_stats(&self) {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            self.hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        
        log::debug!(
            "CoverCache: {} entries, {:.1}% hit rate, {} pending",
            self.cache.len(),
            hit_rate,
            self.pending.len()
        );
    }
}
```

Expected results:
- **Hit rate**: 80-95% (depends on listening patterns)
- **Prefetch effectiveness**: ~70% of next-song loads are cache hits
- **Perceived latency**: <50ms for cached, 100-300ms for uncached

## Common Pitfalls

### 1. Cache Key Mismatch

```rust
// Bug: Different normalization = cache miss
cache.insert(PathBuf::from("Music/song.flac"), data);
cache.get(&PathBuf::from("./Music/song.flac"));  // Miss!

// Solution: Canonicalize paths
let key = path.canonicalize().unwrap_or(path);
```

### 2. Forgetting Pending Cleanup

```rust
// Bug: Pending set grows forever on errors
guard.mark_pending(path.clone());
let result = fetch().await;  // Errors out
// pending.remove() never called!

// Solution: Always clear pending
guard.mark_pending(path.clone());
let result = fetch().await;
{
    let mut guard = cache.write().await;
    guard.pending.remove(&path);  // Always remove
    if let Ok(data) = result {
        guard.cache.put(path, data);
    }
}
```

### 3. Holding Lock During I/O

```rust
// Bad: Lock held during network call
let mut guard = cache.write().await;
let data = client.album_art(&path).await;  // SLOW!
guard.insert(path, data);

// Good: Release lock before I/O
{
    let guard = cache.read().await;
    if guard.contains(&path) { return; }
}  // Lock released

let data = client.album_art(&path).await;  // No lock held

{
    let mut guard = cache.write().await;
    guard.insert(path, data);
}
```

## Related Files

- `src/app/cover_cache.rs` - Cache implementation
- `src/app/main_loop.rs` - Integration and prefetch triggers
