# Async I/O Patterns

## The Problem

TUI applications must remain responsive while performing I/O operations:

- **Network calls**: MPD commands, HTTP requests
- **File system**: Reading cover art, config files
- **System calls**: PipeWire audio configuration
- **Timers**: Progress bar updates, debouncing

Blocking on any of these freezes the UI:

```rust
// Bad: Blocks entire application
fn update_status(&mut self) {
    let status = self.client.status().unwrap();  // Blocks!
    self.render();  // User sees freeze
}
```

## Rust Async Model

Rust uses cooperative multitasking with `async`/`await`:

```rust
// Good: Yields control while waiting
async fn update_status(&mut self) {
    let status = self.client.command(Status).await?;  // Yields here
    // UI can render while waiting
}
```

### Key Concepts

1. **Futures**: Lazy computations that produce a value
2. **Executors**: Runtime that polls futures (tokio)
3. **Wakers**: Mechanism to resume suspended futures
4. **Pinning**: Memory stability for self-referential futures

## tokio::select! for Event-Driven Architecture

The core pattern for responsive TUIs:

```rust
use tokio::select;

loop {
    select! {
        // Multiple event sources, first ready wins
        
        // User input (keyboard)
        _ = tokio::time::sleep(Duration::from_millis(10)) => {
            if crossterm::event::poll(Duration::ZERO)? {
                handle_input().await?;
            }
        }
        
        // MPD state changes (server push)
        event = mpd_events.next() => {
            handle_mpd_event(event).await?;
        }
        
        // Periodic progress updates
        _ = progress_interval.tick() => {
            update_progress().await?;
        }
        
        // Background task completion
        result = cover_art_rx.recv() => {
            handle_cover_art(result)?;
        }
    }
}
```

### Why select! Instead of Multiple Threads?

| Approach | Pros | Cons |
|----------|------|------|
| Threads | True parallelism | Sync overhead, complexity |
| `select!` | Single thread, no locks | Cooperative only |
| Thread + channels | Best of both | More code |

For I/O-bound TUIs, `select!` is usually sufficient and simpler.

## Pattern 1: Fire-and-Forget Background Tasks

For operations where we don't need the result immediately:

```rust
// Cover art loading - result comes via channel
fn spawn_cover_art_loader(
    client: &Client,
    file_path: PathBuf,
    tx: mpsc::Sender<CoverArtMessage>,
) {
    let client = client.clone();
    
    tokio::spawn(async move {
        let data = client.album_art(&file_path.to_string_lossy()).await;
        let _ = tx.send(CoverArtMessage::Loaded(data, file_path)).await;
    });
}
```

### When to Use

- Loading assets (images, fonts)
- Prefetching data
- Logging/telemetry
- Non-critical updates

### Pitfalls

```rust
// Bug: Lost error handling
tokio::spawn(async {
    important_operation().await?;  // Error silently dropped!
});

// Better: Log errors
tokio::spawn(async {
    if let Err(e) = important_operation().await {
        log::error!("Background task failed: {}", e);
    }
});
```

## Pattern 2: spawn_blocking for CPU/Blocking Work

Some operations can't be made async:

```rust
// PipeWire uses its own event loop - can't be async
pub fn set_sample_rate(rate: u32) -> Result<(), String> {
    pipewire::init();
    let mainloop = MainLoopBox::new(None)?;
    // ... runs blocking event loop for ~75ms
}
```

Wrap with `spawn_blocking`:

```rust
pub async fn set_sample_rate_async(rate: u32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_sample_rate(rate))
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}
```

### How spawn_blocking Works

```
┌─────────────────────────────────────────────────┐
│              Tokio Runtime                      │
│  ┌─────────────────────────────────────────┐   │
│  │         Async Task Pool                 │   │
│  │  task1  task2  task3  (non-blocking)    │   │
│  └─────────────────────────────────────────┘   │
│                     │                          │
│                     │ spawn_blocking()         │
│                     ▼                          │
│  ┌─────────────────────────────────────────┐   │
│  │       Blocking Thread Pool              │   │
│  │  thread1: pipewire_call()               │   │
│  │  thread2: file_read()                   │   │
│  └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### When to Use spawn_blocking

- FFI calls to C libraries
- CPU-intensive computation
- Synchronous file I/O (when tokio::fs isn't suitable)
- Third-party blocking APIs

## Pattern 3: Channels for Cross-Task Communication

### mpsc (Multi-Producer, Single-Consumer)

```rust
use tokio::sync::mpsc;

// Create channel
let (tx, mut rx) = mpsc::channel::<Message>(32);

// Producer (can be cloned for multiple producers)
let tx_clone = tx.clone();
tokio::spawn(async move {
    tx_clone.send(Message::Data(data)).await?;
});

// Consumer
while let Some(msg) = rx.recv().await {
    match msg {
        Message::Data(d) => process(d),
        Message::Shutdown => break,
    }
}
```

### oneshot (Single-Use Response)

```rust
use tokio::sync::oneshot;

// Request-response pattern
async fn fetch_with_timeout(client: &Client) -> Result<Status, Error> {
    let (tx, rx) = oneshot::channel();
    
    tokio::spawn({
        let client = client.clone();
        async move {
            let result = client.command(Status).await;
            let _ = tx.send(result);
        }
    });
    
    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(|_| Error::Cancelled)?
}
```

### watch (Latest Value Broadcast)

```rust
use tokio::sync::watch;

// Single producer, multiple consumers, only latest value matters
let (tx, rx) = watch::channel(initial_status);

// Update (overwrites previous)
tx.send(new_status)?;

// Multiple consumers get latest value
let status = rx.borrow().clone();
```

## Pattern 4: Graceful Shutdown

Handle signals without blocking:

```rust
#[cfg(unix)]
async fn run_with_signals(mut app: App) -> Result<()> {
    use tokio::signal::unix::{signal, SignalKind};
    
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    
    loop {
        select! {
            // Normal event handling
            _ = app.tick() => {}
            
            // Graceful shutdown on signals
            _ = sigint.recv() => {
                log::info!("Received SIGINT");
                break;
            }
            _ = sigterm.recv() => {
                log::info!("Received SIGTERM");
                break;
            }
        }
    }
    
    // Cleanup
    app.shutdown().await?;
    Ok(())
}
```

## Pattern 5: Debouncing Rapid Events

Avoid processing every event when they come in bursts:

```rust
use tokio::time::{interval, Duration, Instant};

struct Debouncer {
    last_event: Instant,
    delay: Duration,
}

impl Debouncer {
    fn should_process(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_event) >= self.delay {
            self.last_event = now;
            true
        } else {
            false
        }
    }
}

// Usage: Don't spam MPD with rapid key repeats
if debouncer.should_process() {
    client.command(VolumeUp).await?;
}
```

## Error Handling in Async Context

### Option 1: Propagate with ?

```rust
async fn update(&mut self) -> Result<(), Error> {
    let status = self.client.command(Status).await?;
    let queue = self.client.command(Queue).await?;
    Ok(())
}
```

### Option 2: Log and Continue

```rust
async fn update(&mut self) {
    if let Err(e) = self.client.command(Status).await {
        log::warn!("Status update failed: {}", e);
        // Continue with stale data
    }
}
```

### Option 3: Retry with Backoff

```rust
async fn resilient_connect(addr: &str) -> Result<Client, Error> {
    let mut delay = Duration::from_millis(100);
    
    for attempt in 1..=5 {
        match Client::connect(addr).await {
            Ok(client) => return Ok(client),
            Err(e) => {
                log::warn!("Connect attempt {} failed: {}", attempt, e);
                tokio::time::sleep(delay).await;
                delay *= 2;  // Exponential backoff
            }
        }
    }
    
    Err(Error::ConnectionFailed)
}
```

## Performance Considerations

### 1. Avoid Async in Hot Paths

```rust
// Bad: Async overhead for simple operation
async fn get_cached_value(&self) -> &str {
    &self.cached  // No actual async work!
}

// Good: Just return directly
fn get_cached_value(&self) -> &str {
    &self.cached
}
```

### 2. Batch Async Operations

```rust
// Bad: Sequential awaits
let status = client.command(Status).await?;
let queue = client.command(Queue).await?;
let song = client.command(CurrentSong).await?;

// Good: Parallel with join!
let (status, queue, song) = tokio::join!(
    client.command(Status),
    client.command(Queue),
    client.command(CurrentSong),
);
```

### 3. Bound Channel Sizes

```rust
// Bad: Unbounded can cause memory issues
let (tx, rx) = mpsc::unbounded_channel();

// Good: Bounded with backpressure
let (tx, rx) = mpsc::channel(32);

// Sender blocks if channel full (backpressure)
tx.send(msg).await?;
```

## Common Mistakes

### 1. Blocking in Async Context

```rust
// Bad: Blocks tokio worker thread
async fn process() {
    std::thread::sleep(Duration::from_secs(1));  // NEVER do this!
}

// Good: Use tokio's sleep
async fn process() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### 2. Holding Locks Across Await

```rust
// Bad: Lock held while awaiting
let mut guard = mutex.lock().await;
do_network_call().await;  // Lock held during I/O!
drop(guard);

// Good: Release lock before await
{
    let mut guard = mutex.lock().await;
    let data = guard.clone();
}  // Lock released
do_network_call().await;
```

### 3. Forgetting to Poll Spawned Tasks

```rust
// Task runs but errors are lost
tokio::spawn(async { might_fail().await });

// Better: Join handle for critical tasks
let handle = tokio::spawn(async { might_fail().await });
// Later...
handle.await??;
```

## Related Files

- `src/app/main_loop.rs` - Main select! loop
- `src/pipewire/mod.rs` - spawn_blocking wrappers
- `src/app/cover_cache.rs` - Background loading with channels
