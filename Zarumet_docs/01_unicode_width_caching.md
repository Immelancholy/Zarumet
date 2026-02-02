# Unicode Width Caching

## The Problem

Terminal UIs need to calculate the display width of strings to properly align and truncate text. Unlike ASCII where 1 byte = 1 character = 1 cell, Unicode is complex:

- **Full-width characters** (CJK): 2 cells (e.g., `日` = 2 cells)
- **Combining characters**: 0 cells (e.g., accent marks)
- **Emoji**: 1-2 cells depending on terminal
- **Zero-width characters**: 0 cells (e.g., ZWJ, ZWNJ)

The `unicode-width` crate provides accurate calculations, but it's expensive—O(n) for each string, processing every character.

## Why This Matters

In a music player UI:
- Queue list might have 1000+ songs
- Each song displays: title, artist, album (3 strings)
- Each render frame recalculates all visible items
- At 60 FPS, that's potentially 180,000+ width calculations per second

```rust
// This is called for EVERY string, EVERY frame
fn calculate_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)  // O(n) per call
}
```

## The Solution: LRU Cache

We cache width calculations in a Least Recently Used (LRU) cache:

```rust
pub struct WidthCache {
    cache: LruCache<String, usize>,
    hits: u64,
    misses: u64,
}

impl WidthCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            hits: 0,
            misses: 0,
        }
    }

    pub fn get_width(&mut self, s: &str) -> usize {
        if let Some(&width) = self.cache.get(s) {
            self.hits += 1;
            return width;
        }

        self.misses += 1;
        let width = unicode_width::UnicodeWidthStr::width(s);
        self.cache.put(s.to_owned(), width);
        width
    }
}
```

### Why LRU?

1. **Bounded Memory**: Fixed capacity prevents unbounded growth
2. **Temporal Locality**: Recently used items stay cached (current queue, visible songs)
3. **Automatic Eviction**: Stale entries removed automatically
4. **O(1) Operations**: Hash-based lookup with doubly-linked list for ordering

## Thread-Local Storage

Since ratatui renders on a single thread, we use thread-local storage:

```rust
thread_local! {
    pub static WIDTH_CACHE: RefCell<WidthCache> = RefCell::new(
        WidthCache::new(10_000)
    );
}

// Usage
WIDTH_CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    let width = cache.get_width(&song.title);
    // Use width...
});
```

### Why Thread-Local?

| Approach | Pros | Cons |
|----------|------|------|
| `Arc<Mutex<T>>` | Thread-safe | Lock contention, overhead |
| `Arc<RwLock<T>>` | Multiple readers | Still has overhead |
| `thread_local!` | Zero synchronization | Single-thread only |

For single-threaded rendering, thread-local is optimal—no locks, no atomic operations, just direct memory access.

## Advanced: Truncation Caching

Width calculation alone isn't enough—we also need to truncate strings to fit available space. This compounds the problem:

```rust
// Naive approach: O(n) width check, then O(n) truncation
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let width = unicode_width::UnicodeWidthStr::width(s);
    if width <= max_width {
        return s.to_owned();
    }
    
    // Truncate character by character
    let mut result = String::new();
    let mut current_width = 0;
    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > max_width.saturating_sub(1) {
            break;
        }
        result.push(c);
        current_width += char_width;
    }
    result.push('…');
    result
}
```

### Combined Cache Solution

Cache both width AND pre-truncated versions:

```rust
pub struct WidthCache {
    width_cache: LruCache<String, usize>,
    truncation_cache: LruCache<(String, usize), String>,
}

impl WidthCache {
    pub fn get_truncated(&mut self, s: &str, max_width: usize) -> &str {
        let key = (s.to_owned(), max_width);
        
        if !self.truncation_cache.contains(&key) {
            let truncated = self.compute_truncation(s, max_width);
            self.truncation_cache.put(key.clone(), truncated);
        }
        
        self.truncation_cache.get(&key).unwrap()
    }
}
```

## When to Use This Pattern

**Good candidates for width caching:**
- Song metadata (title, artist, album)
- File paths
- Any user-visible text that's rendered repeatedly

**Poor candidates:**
- One-time strings (log messages)
- Highly dynamic content (timestamps updating every second)
- Very short strings (overhead exceeds benefit)

## Performance Metrics

Our implementation tracks hit rates:

```rust
impl WidthCache {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { return 0.0; }
        self.hits as f64 / total as f64
    }
    
    pub fn log_stats(&self) {
        log::debug!(
            "WidthCache: {} entries, {:.1}% hit rate ({} hits, {} misses)",
            self.cache.len(),
            self.hit_rate() * 100.0,
            self.hits,
            self.misses
        );
    }
}
```

Typical results after stabilization:
- **Hit rate**: 95-99%
- **Memory usage**: ~1-2MB for 10,000 entries
- **CPU reduction**: ~40% of render time

## Implementation Checklist

1. **Choose capacity wisely**: Too small = cache thrashing, too large = memory waste
2. **Track metrics**: Hit rate tells you if cache is effective
3. **Consider key design**: `(string, width)` tuples for truncation vs just `string` for width
4. **Profile first**: Verify width calculation is actually a bottleneck
5. **Test with real data**: Synthetic benchmarks may not reflect actual usage patterns

## Related Files

- `src/ui/width_cache.rs` - Main implementation
- `src/ui/utils.rs` - Helper functions using the cache
- `src/ui/widgets/queue.rs` - Consumer of cached truncation
