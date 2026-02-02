# Zarumet Performance Optimization Documentation

This documentation covers the middle-to-advanced performance optimizations and best practices implemented in Zarumet, a terminal-based MPD client written in Rust.

## Document Index

### Performance Optimizations

1. **[Unicode Width Caching](./01_unicode_width_caching.md)** - LRU caching for expensive Unicode width calculations
2. **[Render Caching & String Interning](./02_render_caching.md)** - Pre-computed display strings and string deduplication
3. **[Dirty Region Rendering](./03_dirty_region_rendering.md)** - Conditional rendering based on state changes
4. **[Async I/O Patterns](./04_async_io_patterns.md)** - Non-blocking operations with tokio
5. **[Cover Art Prefetching](./05_cover_art_prefetching.md)** - LRU cache with predictive loading
6. **[Thread-Local vs Shared State](./06_thread_local_vs_shared.md)** - Choosing the right synchronization primitive
7. **[MPD Protocol Optimization](./07_mpd_protocol_optimization.md)** - Reducing network round-trips
8. **[Memory-Efficient Data Structures](./08_memory_efficient_structures.md)** - Choosing appropriate collections

### Features & Architecture

9. **[Sequential Key Bindings](./09_sequential_key_bindings.md)** - Vim-style multi-key command sequences
10. **[Configuration Validation](./10_configuration_validation.md)** - Fuzzy matching for "did you mean?" suggestions
11. **[Terminal Graphics Protocols](./11_terminal_graphics_protocols.md)** - Cover art rendering with Kitty/Sixel/iTerm2
12. **[Logging Architecture](./12_logging_architecture.md)** - Structured file logging with rotation

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Event Loop                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ MPD Events  │  │ User Input  │  │ Progress Timer          │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
│         │                │                     │               │
│         ▼                ▼                     ▼               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Dirty Flags                          │   │
│  │  queue | status | progress | cover_art | key_sequence   │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                   │
│                            ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Conditional Render (if any_dirty)          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │   │
│  │  │Width Cache  │  │Render Cache │  │Cover Cache  │     │   │
│  │  │(thread-loc) │  │(thread-loc) │  │(Arc<RwLock>)│     │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘     │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Key Performance Principles

### 1. Avoid Redundant Work
- Cache expensive computations (Unicode width, formatted strings)
- Use dirty flags to skip unchanged regions
- Batch MPD commands when possible

### 2. Non-Blocking I/O
- All network operations are async
- Blocking operations (PipeWire) run on dedicated threads
- Background tasks for cover art loading

### 3. Memory Efficiency
- LRU eviction for bounded caches
- String interning for repeated values
- Lazy loading for large data sets

### 4. Responsive UI
- Event-driven architecture (no polling loops)
- Immediate visual feedback
- Progressive loading patterns

## Performance Impact Summary

| Optimization | CPU Reduction | Latency Improvement | Memory Trade-off |
|-------------|---------------|---------------------|------------------|
| Width Caching | ~40% render time | Imperceptible | +1-2MB |
| Render Caching | ~20% render time | Imperceptible | +0.5MB |
| Dirty Regions | ~70% idle CPU | N/A | +64 bytes |
| Async PipeWire | N/A | -75ms per call | None |
| Cover Prefetch | N/A | -200ms perceived | +5-10MB |

## Prerequisites for Understanding

- Familiarity with Rust ownership and borrowing
- Basic understanding of async/await patterns
- Knowledge of TUI rendering concepts
- Understanding of client-server protocols (MPD)
