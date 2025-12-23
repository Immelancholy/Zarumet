# MPD Protocol Optimization

## The Problem

MPD (Music Player Daemon) uses a text-based protocol over TCP/Unix sockets. Each command involves:

1. Send command text
2. Wait for server processing
3. Receive response
4. Parse response

Network latency adds up with multiple round-trips:

```rust
// Naive approach: 4 round-trips
let status = client.command(Status).await?;       // RTT 1
let current = client.command(CurrentSong).await?; // RTT 2
let queue = client.command(Queue).await?;         // RTT 3
let outputs = client.command(Outputs).await?;     // RTT 4
// Total: ~40-80ms on local, 200-400ms on network
```

## Solution 1: Idle/Subsystem Notifications

Instead of polling, use MPD's idle system:

```rust
// Bad: Polling every 100ms
loop {
    let status = client.command(Status).await?;
    if status != old_status {
        update_ui();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
}

// Good: Wait for server notifications
loop {
    let event = client.idle().await?;  // Blocks until change
    match event {
        Subsystem::Player => update_player_status(),
        Subsystem::Queue => update_queue(),
        Subsystem::Mixer => update_volume(),
        // ...
    }
}
```

### Benefits

- Zero CPU when idle
- Instant response to changes
- Reduced network traffic
- Server-authoritative updates

### Implementation

```rust
let (client, mut events) = Client::connect(addr).await?;

// Main loop uses select! to handle events
loop {
    tokio::select! {
        event = events.next() => {
            match event {
                Some(ConnectionEvent::SubsystemChange(sub)) => {
                    handle_subsystem_change(sub).await?;
                }
                Some(ConnectionEvent::ConnectionClosed(_)) => break,
                None => break,
            }
        }
        // Other event sources...
    }
}
```

## Solution 2: Selective Updates Based on Subsystem

Not all changes require fetching all data:

```rust
async fn handle_subsystem_change(&mut self, subsystem: Subsystem) {
    match subsystem {
        // Player state: Need status, maybe current song
        Subsystem::Player => {
            self.run_optimized_updates(false, true).await?;
        }
        
        // Volume changed: Only need status
        Subsystem::Mixer => {
            self.update_status_only().await?;
        }
        
        // Playback options: Only need status
        Subsystem::Options => {
            self.update_status_only().await?;
        }
        
        // Queue changed: Need full update
        Subsystem::Queue => {
            self.run_updates().await?;
        }
        
        // Database, outputs, etc.: Usually no UI impact
        Subsystem::Database | Subsystem::Output => {
            log::debug!("Ignoring subsystem: {:?}", subsystem);
        }
    }
}
```

### Status-Only Update

```rust
impl App {
    async fn update_status_only(&mut self, client: &Client) -> Result<()> {
        let status = client.command(commands::Status).await?;
        
        // Update only status-related fields
        if let Some(ref mut song) = self.current_song {
            song.update_playback_info(Some(status.state), None);
        }
        
        self.mpd_status = Some(status);
        self.dirty.mark_status();
        
        Ok(())
    }
}
```

### Optimized Update (Status + Maybe Song)

```rust
impl App {
    async fn run_optimized_updates(
        &mut self,
        client: &Client,
        force_queue: bool,
        check_song_change: bool,
    ) -> Result<()> {
        let status = client.command(commands::Status).await?;
        
        // Only fetch current song if it might have changed
        let song_changed = check_song_change && 
            status.current_song != self.mpd_status.as_ref().map(|s| s.current_song);
        
        if song_changed {
            self.update_current_song(client).await?;
            self.dirty.mark_current_song();
            self.dirty.mark_cover_art();
        }
        
        if force_queue {
            self.update_queue(client).await?;
            self.dirty.mark_queue();
        }
        
        self.mpd_status = Some(status);
        self.dirty.mark_status();
        
        Ok(())
    }
}
```

## Solution 3: Use Cached Status for User Actions

Many user actions need current status to compute new values:

```rust
// Bad: Extra round-trip for every volume change
async fn volume_up(&self, client: &Client) {
    let status = client.command(Status).await?;  // Unnecessary!
    let new_vol = status.volume + 5;
    client.command(SetVolume(new_vol)).await?;
}

// Good: Use cached status
async fn volume_up(&self, client: &Client, cached_status: &Status) {
    let new_vol = cached_status.volume + 5;
    client.command(SetVolume(new_vol)).await?;
}
```

### Implementation Pattern

```rust
impl MPDAction {
    pub async fn execute(
        &self,
        client: &Client,
        config: &Config,
        cached_status: Option<&Status>,  // Pass cached status
    ) -> Result<()> {
        match self {
            MPDAction::VolumeUp => {
                let current = cached_status
                    .map(|s| s.volume)
                    .unwrap_or_else(|| {
                        // Fallback: fetch if no cache
                        client.command(Status).await?.volume
                    });
                let new_vol = (current + config.volume_increment).min(100);
                client.command(SetVolume(new_vol)).await?;
            }
            // ...
        }
    }
}
```

## Solution 4: Progress Updates Without Full Status

Progress bar updates are frequent but only need elapsed time:

```rust
// Bad: Full status every 500ms
loop {
    let status = client.command(Status).await?;
    update_progress(status.elapsed, status.duration);
    tokio::time::sleep(Duration::from_millis(500)).await;
}

// Good: Only fetch when actually playing
if self.mpd_status.as_ref().map(|s| s.state) == Some(PlayState::Playing) {
    let status = client.command(Status).await?;
    
    // Only update progress-related fields
    if let Some(ref mut song) = self.current_song {
        let progress = match (status.elapsed, status.duration) {
            (Some(e), Some(d)) => Some(e.as_secs_f64() / d.as_secs_f64()),
            _ => None,
        };
        song.update_time_info(status.elapsed, status.duration);
    }
    
    self.dirty.mark_progress();  // Only progress dirty, not full status
}
```

## Solution 5: Command Batching

When multiple updates are needed, batch them:

```rust
// MPD supports command lists for batching
// Note: mpd_client crate may handle this internally

// Conceptually:
// command_list_begin
// status
// currentsong
// playlistinfo
// command_list_end
// 
// Single round-trip returns all results
```

## Solution 6: Song Change Detection

Detect song changes without fetching song data:

```rust
impl App {
    fn song_changed(&self, new_status: &Status) -> bool {
        let old_song_id = self.mpd_status
            .as_ref()
            .and_then(|s| s.current_song.as_ref())
            .map(|s| s.id);
        
        let new_song_id = new_status.current_song
            .as_ref()
            .map(|s| s.id);
        
        old_song_id != new_song_id
    }
}
```

This uses the song ID from status (which we already fetched) rather than fetching the full song info.

## Connection Management

### Reconnection Strategy

```rust
async fn maintain_connection(addr: &str) -> Result<()> {
    let mut backoff = Duration::from_millis(100);
    
    loop {
        match Client::connect(addr).await {
            Ok((client, events)) => {
                backoff = Duration::from_millis(100);  // Reset backoff
                
                if let Err(e) = run_client(client, events).await {
                    log::warn!("Connection error: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Connect failed: {}, retrying in {:?}", e, backoff);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}
```

### Connection Pooling (If Needed)

For high-throughput scenarios:

```rust
// Usually not needed for TUI apps, but available pattern
pub struct ClientPool {
    clients: Vec<Client>,
    available: tokio::sync::Semaphore,
}

impl ClientPool {
    pub async fn get(&self) -> PooledClient {
        let permit = self.available.acquire().await.unwrap();
        // Return client with permit that releases on drop
    }
}
```

## Benchmarks

### Before Optimization

| Operation | Commands | RTT | Time |
|-----------|----------|-----|------|
| Full refresh | 4 | 4 | ~40ms |
| Volume change | 2 | 2 | ~20ms |
| Progress update | 1 | 1 | ~10ms |

### After Optimization

| Operation | Commands | RTT | Time |
|-----------|----------|-----|------|
| Full refresh | 4 | 4 | ~40ms |
| Status-only | 1 | 1 | ~10ms |
| Volume (cached) | 1 | 1 | ~10ms |
| Progress (playing) | 1 | 1 | ~10ms |
| Progress (paused) | 0 | 0 | ~0ms |

### Idle CPU Usage

| Approach | CPU Usage |
|----------|-----------|
| Polling @ 100ms | 5-10% |
| Idle + events | <0.5% |

## Anti-Patterns

### 1. Fetching Everything Always

```rust
// Bad: Fetch all data on every event
async fn on_event(&mut self) {
    self.status = client.command(Status).await?;
    self.queue = client.command(Queue).await?;  // 1000 songs!
    self.song = client.command(CurrentSong).await?;
    self.outputs = client.command(Outputs).await?;
}
```

### 2. Ignoring Subsystem Hints

```rust
// Bad: Same response to all events
async fn on_subsystem(&mut self, _sub: Subsystem) {
    self.full_refresh().await?;  // Wasteful!
}
```

### 3. Not Caching Across Actions

```rust
// Bad: Fetch status for each action in sequence
async fn handle_macro(&mut self) {
    self.volume_up().await?;   // Fetches status
    self.volume_up().await?;   // Fetches status again
    self.volume_up().await?;   // And again
}
```

## Related Files

- `src/app/mpd_updates.rs` - Optimized update methods
- `src/app/mpd_handler.rs` - Action execution with cached status
- `src/app/main_loop.rs` - Event handling and subsystem routing
