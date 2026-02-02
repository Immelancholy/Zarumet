# Thread-Local vs Shared State

## The Decision Matrix

Choosing the right synchronization primitive is critical for performance:

```
                        Single Thread?
                             │
              ┌──────────────┴──────────────┐
              │                             │
             Yes                           No
              │                             │
              ▼                             ▼
        thread_local!               Shared State Needed
              │                             │
              │                 ┌───────────┴───────────┐
              │                 │                       │
              │           Read-Heavy?              Write-Heavy?
              │                 │                       │
              │                 ▼                       ▼
              │            RwLock                   Mutex
              │                 │                       │
              │         ┌───────┴───────┐              │
              │         │               │              │
              │    Async Context?  Sync Only?         │
              │         │               │              │
              │         ▼               ▼              │
              │   tokio::sync::   std::sync::         │
              │     RwLock          RwLock            │
              │                                       │
              └───────────────────────────────────────┘
```

## Option 1: Thread-Local (RefCell)

### When to Use

- Single-threaded context (TUI rendering)
- No need to share across threads
- Maximum performance required

### Implementation

```rust
use std::cell::RefCell;

pub struct WidthCache {
    cache: LruCache<String, usize>,
}

// Thread-local storage - no synchronization needed
thread_local! {
    pub static WIDTH_CACHE: RefCell<WidthCache> = RefCell::new(
        WidthCache::new(10_000)
    );
}

// Usage
fn calculate_width(s: &str) -> usize {
    WIDTH_CACHE.with(|cache| {
        cache.borrow_mut().get_width(s)
    })
}
```

### Performance Characteristics

| Operation | Cost |
|-----------|------|
| Access | Single pointer dereference |
| Borrow | Runtime check (~1 ns) |
| No contention | Guaranteed |

### Pitfalls

```rust
// Bug: Panic on nested borrow
WIDTH_CACHE.with(|cache| {
    let guard = cache.borrow_mut();
    
    // This panics! Already borrowed mutably
    WIDTH_CACHE.with(|cache2| {
        let guard2 = cache2.borrow_mut();  // PANIC!
    });
});

// Solution: Keep borrows short
WIDTH_CACHE.with(|cache| {
    let width = cache.borrow_mut().get_width(s);
    // guard dropped here
    
    // Now safe to borrow again
    let other = cache.borrow().peek_width(s);
});
```

## Option 2: Arc<Mutex<T>>

### When to Use

- Shared across async tasks
- Write-heavy access pattern
- Simple synchronization needs

### Implementation

```rust
use std::sync::{Arc, Mutex};

pub struct SharedState {
    data: Vec<Item>,
}

pub type SharedStateHandle = Arc<Mutex<SharedState>>;

pub fn new_shared_state() -> SharedStateHandle {
    Arc::new(Mutex::new(SharedState { data: Vec::new() }))
}

// Usage
async fn update_state(state: SharedStateHandle) {
    let mut guard = state.lock().unwrap();
    guard.data.push(new_item);
}
```

### Performance Characteristics

| Operation | Cost |
|-----------|------|
| Lock acquisition | Atomic CAS (~10-50 ns uncontended) |
| Contention | Thread blocks waiting |
| Memory | Arc overhead (~16 bytes) |

### Async Considerations

```rust
// std::sync::Mutex - OK for short critical sections
let guard = mutex.lock().unwrap();
// Do quick work
drop(guard);
// Now safe to await

// DANGER: Holding std::sync::Mutex across await
let guard = mutex.lock().unwrap();
async_operation().await;  // Other tasks can't acquire lock!
drop(guard);
```

## Option 3: Arc<RwLock<T>>

### When to Use

- Shared across async tasks
- Read-heavy access pattern
- Multiple concurrent readers beneficial

### Implementation

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CoverCache {
    cache: LruCache<PathBuf, CoverArtEntry>,
}

pub type SharedCoverCache = Arc<RwLock<CoverCache>>;

// Multiple readers can proceed in parallel
async fn check_cache(cache: &SharedCoverCache, path: &PathBuf) -> bool {
    let guard = cache.read().await;
    guard.contains(path)
}

// Writers get exclusive access
async fn insert_cache(cache: &SharedCoverCache, path: PathBuf, data: Vec<u8>) {
    let mut guard = cache.write().await;
    guard.insert(path, data);
}
```

### Performance Characteristics

| Operation | Cost |
|-----------|------|
| Read lock | Atomic increment (~10 ns) |
| Write lock | Wait for all readers |
| Multiple readers | Parallel access |

### tokio vs std RwLock

```rust
// std::sync::RwLock - Cannot hold across await points
let guard = std_rwlock.read().unwrap();
some_async_fn().await;  // UNDEFINED BEHAVIOR or DEADLOCK

// tokio::sync::RwLock - Safe across await points
let guard = tokio_rwlock.read().await;
some_async_fn().await;  // Safe, but consider releasing first
```

## Option 4: Atomic Types

### When to Use

- Single values (counters, flags)
- Lock-free required
- Simple read-modify-write operations

### Implementation

```rust
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

pub struct Stats {
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    is_enabled: AtomicBool,
}

impl Stats {
    pub fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        hits as f64 / (hits + misses) as f64
    }
}
```

### Memory Ordering

| Ordering | Use Case |
|----------|----------|
| `Relaxed` | Statistics, counters (no synchronization) |
| `Acquire/Release` | Producer-consumer patterns |
| `SeqCst` | When in doubt (strongest guarantee) |

## Option 5: Channels (Message Passing)

### When to Use

- Communication between tasks
- Decoupled producer/consumer
- No shared mutable state

### Implementation

```rust
use tokio::sync::mpsc;

enum CacheMessage {
    Insert(PathBuf, Vec<u8>),
    Query(PathBuf, oneshot::Sender<Option<Vec<u8>>>),
}

// Cache manager task
async fn cache_manager(mut rx: mpsc::Receiver<CacheMessage>) {
    let mut cache = LruCache::new(100);
    
    while let Some(msg) = rx.recv().await {
        match msg {
            CacheMessage::Insert(path, data) => {
                cache.put(path, data);
            }
            CacheMessage::Query(path, response) => {
                let result = cache.get(&path).cloned();
                let _ = response.send(result);
            }
        }
    }
}

// Usage
async fn query_cache(tx: &mpsc::Sender<CacheMessage>, path: PathBuf) -> Option<Vec<u8>> {
    let (response_tx, response_rx) = oneshot::channel();
    tx.send(CacheMessage::Query(path, response_tx)).await.ok()?;
    response_rx.await.ok()?
}
```

### Pros and Cons

| Pros | Cons |
|------|------|
| No shared state | Message overhead |
| Clear ownership | Latency for queries |
| Easy to reason about | More boilerplate |

## Zarumet's Choices

### 1. Width Cache: Thread-Local

```rust
// Single-threaded rendering, maximum performance
thread_local! {
    pub static WIDTH_CACHE: RefCell<WidthCache> = ...
}
```

**Reasoning**: 
- Only accessed during render (single thread)
- Called thousands of times per frame
- No need for cross-thread access

### 2. Render Cache: Thread-Local

```rust
thread_local! {
    pub static RENDER_CACHE: RefCell<RenderCache> = ...
}
```

**Reasoning**:
- Same as width cache
- Tight coupling with render loop

### 3. Cover Art Cache: Arc<RwLock>

```rust
pub type SharedCoverCache = Arc<RwLock<CoverCache>>;
```

**Reasoning**:
- Background tasks load cover art
- Main thread reads cache
- Read-heavy pattern (many checks, few inserts)

### 4. Dirty Flags: Direct Field

```rust
pub struct App {
    dirty: DirtyFlags,  // No wrapper
}
```

**Reasoning**:
- Single owner (App)
- Only accessed by main loop
- No synchronization needed

## Performance Comparison

Benchmark: 1 million cache accesses

| Approach | Time (ms) | Relative |
|----------|-----------|----------|
| thread_local! + RefCell | 45 | 1.0x |
| Arc<Mutex> (uncontended) | 120 | 2.7x |
| Arc<RwLock> read | 85 | 1.9x |
| Arc<RwLock> write | 150 | 3.3x |
| Channel round-trip | 2500 | 55x |

## Migration Patterns

### Thread-Local → Shared (if needs change)

```rust
// Before: Thread-local
thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::new());
}

fn use_cache() {
    CACHE.with(|c| c.borrow_mut().get(key))
}

// After: Shared
lazy_static! {
    static ref CACHE: Arc<RwLock<Cache>> = Arc::new(RwLock::new(Cache::new()));
}

async fn use_cache() {
    CACHE.read().await.get(key)
}
```

### Shared → Thread-Local (for performance)

```rust
// Before: Shared (but actually single-threaded)
let cache = Arc::new(Mutex::new(Cache::new()));

// After: Thread-local (faster)
thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::new());
}
```

## Decision Checklist

1. **Is it accessed from multiple threads?**
   - No → `thread_local!` or owned field
   - Yes → Continue...

2. **Is it async context?**
   - Yes → Use `tokio::sync::*`
   - No → Use `std::sync::*`

3. **What's the access pattern?**
   - Read-heavy → `RwLock`
   - Write-heavy or simple → `Mutex`
   - Single value → `Atomic*`
   - Decoupled → Channels

4. **How critical is latency?**
   - Very → Prefer lock-free or thread-local
   - Moderate → `RwLock` is fine
   - Not critical → Channels for simplicity

## Related Files

- `src/ui/width_cache.rs` - Thread-local example
- `src/ui/render_cache.rs` - Thread-local example  
- `src/app/cover_cache.rs` - Arc<RwLock> example
- `src/app/mod.rs` - Direct field example
