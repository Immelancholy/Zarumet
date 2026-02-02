# Memory-Efficient Data Structures

## The Challenge

A music library can contain:
- 10,000+ songs
- 1,000+ albums  
- 500+ artists
- Metadata: titles, artists, albums, paths, durations

Naive storage quickly consumes hundreds of megabytes:

```rust
// Naive: Each song owns all its strings
pub struct Song {
    title: String,      // "Highway to Hell" - 16 bytes + heap
    artist: String,     // "AC/DC" - repeated 100 times!
    album: String,      // "Highway to Hell" - repeated 10 times!
    path: PathBuf,      // Unique per song
    duration: Duration,
}
// 10,000 songs × ~500 bytes = ~5MB minimum
// But with string duplication: ~15-20MB
```

## Strategy 1: Lazy Loading

Don't load everything upfront:

```rust
pub struct LazyLibrary {
    // Only artist names loaded initially (~50KB for 500 artists)
    pub artists: Vec<String>,
    
    // Albums loaded on-demand per artist
    artist_albums: HashMap<usize, Vec<Album>>,
    
    // Tracks loaded on-demand per album
    album_tracks: HashMap<AlbumKey, Vec<Track>>,
}

impl LazyLibrary {
    /// Initial load: Just artist names
    pub async fn init(client: &Client) -> Result<Self> {
        let artists = client.command(ListArtists).await?;
        Ok(Self {
            artists,
            artist_albums: HashMap::new(),
            album_tracks: HashMap::new(),
        })
    }
    
    /// Load albums when artist is selected
    pub async fn load_artist(&mut self, client: &Client, artist_idx: usize) -> Result<()> {
        if self.artist_albums.contains_key(&artist_idx) {
            return Ok(());  // Already loaded
        }
        
        let artist = &self.artists[artist_idx];
        let albums = client.command(ListAlbums(artist)).await?;
        self.artist_albums.insert(artist_idx, albums);
        Ok(())
    }
}
```

### Memory Savings

| Approach | Memory for 10K songs |
|----------|---------------------|
| Eager load all | ~15-20MB |
| Lazy (artists only) | ~50KB initial |
| Lazy (1 artist expanded) | ~50KB + ~500KB |

## Strategy 2: Structural Sharing

Deduplicate repeated data:

```rust
use std::sync::Arc;

pub struct Song {
    title: String,           // Unique per song
    artist: Arc<str>,        // Shared among songs by same artist
    album: Arc<str>,         // Shared among songs in same album
    path: PathBuf,           // Unique per song
}

// Artist "AC/DC" appears on 100 songs:
// Before: 100 × "AC/DC" string allocations
// After: 1 × Arc<str> + 100 × Arc clone (pointer copy)
```

### Implementation

```rust
pub struct LibraryBuilder {
    artist_pool: HashMap<String, Arc<str>>,
    album_pool: HashMap<String, Arc<str>>,
}

impl LibraryBuilder {
    pub fn intern_artist(&mut self, artist: &str) -> Arc<str> {
        if let Some(existing) = self.artist_pool.get(artist) {
            Arc::clone(existing)
        } else {
            let interned: Arc<str> = artist.into();
            self.artist_pool.insert(artist.to_owned(), Arc::clone(&interned));
            interned
        }
    }
    
    pub fn build_song(&mut self, raw: RawSong) -> Song {
        Song {
            title: raw.title,
            artist: self.intern_artist(&raw.artist),
            album: self.intern_album(&raw.album),
            path: raw.path,
        }
    }
}
```

### Memory Savings

For a library with 500 artists, 1000 albums, 10000 songs:
- Before: 10000 × (artist + album strings) ≈ 1.5MB
- After: 500 + 1000 unique strings + 10000 Arc pointers ≈ 200KB

## Strategy 3: SmallVec for Small Collections

Avoid heap allocation for small, common cases:

```rust
use smallvec::SmallVec;

pub struct Album {
    name: String,
    // Most albums have <20 tracks; avoid heap for common case
    tracks: SmallVec<[Track; 16]>,
}

// If tracks.len() <= 16: All stored inline (no heap allocation)
// If tracks.len() > 16: Spills to heap automatically
```

### When to Use

| Collection Size | Best Choice |
|-----------------|-------------|
| Always 0-4 items | `ArrayVec<[T; 4]>` or tuple |
| Usually <16, sometimes more | `SmallVec<[T; 16]>` |
| Usually >16 | `Vec<T>` |
| Unknown/large | `Vec<T>` |

## Strategy 4: Compact String Representations

For fixed-format data, use compact representations:

```rust
// Duration as seconds (u32) instead of std::time::Duration
pub struct CompactSong {
    duration_secs: u32,  // 4 bytes vs 16 bytes for Duration
}

// File type as enum instead of String
#[repr(u8)]
pub enum FileType {
    Flac = 0,
    Mp3 = 1,
    Ogg = 2,
    Opus = 3,
    M4a = 4,
    Wav = 5,
    Other = 255,
}
// 1 byte vs ~8+ bytes for String
```

### Path Optimization

```rust
// Store path relative to music root
pub struct CompactSong {
    // Instead of: "/home/user/Music/Artist/Album/song.flac"
    // Store: "Artist/Album/song.flac"
    relative_path: PathBuf,
}

// Reconstruct full path when needed:
impl CompactSong {
    pub fn full_path(&self, music_root: &Path) -> PathBuf {
        music_root.join(&self.relative_path)
    }
}
```

## Strategy 5: Index-Based References

Instead of storing copies, store indices:

```rust
pub struct Library {
    artists: Vec<String>,
    albums: Vec<Album>,
    songs: Vec<Song>,
}

pub struct Song {
    title: String,
    artist_idx: u16,  // Index into library.artists
    album_idx: u16,   // Index into library.albums
    // ...
}

impl Library {
    pub fn song_artist(&self, song: &Song) -> &str {
        &self.artists[song.artist_idx as usize]
    }
}
```

### Trade-offs

| Approach | Memory | Access Speed | Complexity |
|----------|--------|--------------|------------|
| Owned strings | High | O(1) | Low |
| Arc<str> | Medium | O(1) | Low |
| Index-based | Low | O(1) + indirection | Medium |

## Strategy 6: Efficient Queue Representation

The play queue is a hot path—optimize for common operations:

```rust
pub struct Queue {
    // VecDeque for efficient front/back operations
    songs: VecDeque<QueueEntry>,
}

pub struct QueueEntry {
    // Only store what's needed for display
    song_id: u32,           // MPD's song ID
    position: u16,          // Queue position (0-65535)
    
    // Display info (could be lazy-loaded)
    display: Option<QueueDisplay>,
}

pub struct QueueDisplay {
    title: String,
    artist: String,
    duration_secs: u32,
}
```

### Queue Operations

```rust
impl Queue {
    // O(1) operations
    pub fn current(&self) -> Option<&QueueEntry> {
        self.songs.front()
    }
    
    pub fn remove_current(&mut self) -> Option<QueueEntry> {
        self.songs.pop_front()
    }
    
    pub fn add_to_end(&mut self, entry: QueueEntry) {
        self.songs.push_back(entry);
    }
    
    // O(n) but rare
    pub fn move_entry(&mut self, from: usize, to: usize) {
        if let Some(entry) = self.songs.remove(from) {
            self.songs.insert(to, entry);
        }
    }
}
```

## Strategy 7: Flyweight Pattern for Display

Separate intrinsic (shared) from extrinsic (context-specific) state:

```rust
// Intrinsic: Shared song data
pub struct SongData {
    title: Arc<str>,
    artist: Arc<str>,
    album: Arc<str>,
    duration: Duration,
}

// Extrinsic: Context-specific display state
pub struct QueueItemView<'a> {
    data: &'a SongData,
    position: usize,
    is_playing: bool,
    is_selected: bool,
}

impl<'a> QueueItemView<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Use self.data for content
        // Use self.is_playing/is_selected for styling
    }
}
```

## Memory Profiling

### Track Allocations

```rust
// Use a global allocator wrapper for debugging
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: cap::Cap<std::alloc::System> = cap::Cap::new(
    std::alloc::System,
    usize::MAX,
);

// Check memory usage
fn log_memory() {
    #[cfg(debug_assertions)]
    log::debug!("Allocated: {} bytes", ALLOC.allocated());
}
```

### Collection Size Estimation

```rust
impl Library {
    pub fn memory_estimate(&self) -> usize {
        let artists_size: usize = self.artists.iter()
            .map(|s| std::mem::size_of::<String>() + s.len())
            .sum();
        
        let songs_size = self.songs.len() * std::mem::size_of::<Song>();
        
        // Include Vec overhead
        let vec_overhead = std::mem::size_of::<Vec<String>>() * 3;
        
        artists_size + songs_size + vec_overhead
    }
}
```

## Benchmarks

### Memory Usage Comparison

| Approach | 10K Songs | 100K Songs |
|----------|-----------|------------|
| Naive (owned strings) | 15MB | 150MB |
| Arc string interning | 8MB | 60MB |
| Index-based | 5MB | 40MB |
| Lazy loading | 50KB-2MB | 50KB-20MB |

### CPU Impact

| Approach | Access Time | Notes |
|----------|-------------|-------|
| Owned strings | Fastest | Direct access |
| Arc<str> | ~Same | Pointer dereference |
| Index-based | +5-10ns | Extra indirection |
| Lazy loading | Variable | Network I/O on miss |

## Choosing the Right Approach

```
                        Data Size?
                            │
            ┌───────────────┴───────────────┐
            │                               │
         Small                           Large
        (<1000)                         (>10000)
            │                               │
            ▼                               ▼
     Simple Owned                    Consider:
       Strings                       - Lazy loading
                                    - String interning
                                    - Index-based refs
                                            │
                                            ▼
                                    Access Pattern?
                                            │
                        ┌───────────────────┴───────────────────┐
                        │                                       │
                  Sequential                              Random Access
                   (iteration)                            (lookups)
                        │                                       │
                        ▼                                       ▼
                  Index-based                            Arc<str> or
                  (best cache                            HashMap lookup
                   locality)
```

## Related Files

- `src/song.rs` - Song and library structures
- `src/ui/widgets/queue.rs` - Queue display
- `src/app/mod.rs` - App state management
