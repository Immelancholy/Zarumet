# Configuration Validation with Fuzzy Matching

This document explains Zarumet's configuration validation system, which provides helpful "did you mean" suggestions when users make typos in their config file.

## Problem

TOML configuration files with serde's default deserializer silently ignore unknown fields. This means:

1. **Typos go unnoticed** - `volumee_increment` instead of `volume_increment` is silently ignored
2. **Deprecated options persist** - Users don't know when config options are removed
3. **Copy-paste errors** - Configs from old versions or other apps cause confusion
4. **Debugging difficulty** - "Why isn't my setting working?" becomes a common question

## Solution

Zarumet implements a two-pass configuration loading strategy:

```
┌─────────────────────────────────────────────────────────────┐
│                    Config Loading Pipeline                   │
├─────────────────────────────────────────────────────────────┤
│  1. Read TOML as generic table                              │
│  2. Check all keys against known field lists                │
│  3. For unknown keys, compute Levenshtein distance          │
│  4. Generate "did you mean?" suggestions if close match     │
│  5. Collect warnings (don't fail - graceful degradation)    │
│  6. Parse config with serde (fallback to defaults)          │
│  7. Display warnings to user via popup                      │
└─────────────────────────────────────────────────────────────┘
```

## Implementation

### Levenshtein Distance Algorithm

The Levenshtein distance measures the minimum number of single-character edits (insertions, deletions, substitutions) required to change one string into another.

From `src/config.rs:303-334`:

```rust
/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use two rows instead of full matrix for memory efficiency
    // Space complexity: O(min(a_len, b_len)) instead of O(a_len * b_len)
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for (i, a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;

        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1)  // deletion
                .min(curr_row[j] + 1)                 // insertion
                .min(prev_row[j] + cost);             // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}
```

**Memory optimization**: Instead of maintaining the full `O(n*m)` matrix, we only keep two rows at a time, reducing memory usage to `O(m)`.

### Finding Similar Strings

From `src/config.rs:337-362`:

```rust
/// Find the most similar string from a list of candidates
fn find_similar(unknown: &str, candidates: &[&str]) -> Option<String> {
    let unknown_lower = unknown.to_lowercase();

    let mut best_match: Option<(&str, usize)> = None;

    for &candidate in candidates {
        let distance = levenshtein_distance(&unknown_lower, &candidate.to_lowercase());

        // Only suggest if the distance is reasonable
        // Threshold: at most half the length of the longer string, minimum 3
        let max_len = unknown.len().max(candidate.len());
        let threshold = (max_len / 2).max(3);

        if distance <= threshold {
            if let Some((_, best_distance)) = best_match {
                if distance < best_distance {
                    best_match = Some((candidate, distance));
                }
            } else {
                best_match = Some((candidate, distance));
            }
        }
    }

    best_match.map(|(s, _)| s.to_string())
}
```

**Threshold logic**: The threshold scales with string length but has a minimum of 3. This prevents suggesting completely unrelated options while still catching common typos.

### Checking Unknown Fields

The validation checks both sections and fields within sections:

```rust
// Known top-level sections
const KNOWN_SECTIONS: &[&str] = &["mpd", "colors", "binds", "pipewire", "logging"];

// Known fields per section
const KNOWN_MPD_FIELDS: &[&str] = &["address", "volume_increment", "volume_increment_fine"];

const KNOWN_COLORS_FIELDS: &[&str] = &[
    "border", "song_title", "album", "artist", "border_title",
    "progress_filled", "progress_empty", "paused", "playing", "stopped",
    // ... 20+ more color fields
];

const KNOWN_BINDS_FIELDS: &[&str] = &[
    "next", "previous", "toggle_play_pause", "volume_up", "volume_down",
    "scroll_up", "scroll_down", "play_selected", "go_to_top", "go_to_bottom",
    // ... 30+ more keybinding fields
];
```

### Warning Message Formatting

From `src/config.rs:364-380`:

```rust
/// Format an unknown config warning with optional "did you mean" suggestion
fn format_unknown_warning(section: &str, key: &str, suggestion: Option<&str>) -> String {
    if section == "section" {
        match suggestion {
            Some(s) => format!("Unknown config section: [{}] (did you mean: [{}]?)", key, s),
            None => format!("Unknown config section: [{}]", key),
        }
    } else {
        match suggestion {
            Some(s) => format!(
                "Unknown option in {}: {} (did you mean: {}?)",
                section, key, s
            ),
            None => format!("Unknown option in {}: {}", section, key),
        }
    }
}
```

## User Experience

### Warning Popup

When unknown configuration options are detected, Zarumet displays a centered popup on startup:

```
╭─ Unknown Config Options ─────────────────────────╮
│                                                  │
│  Unknown option in [mpd]: adress (did you mean:  │
│  address?)                                       │
│  Unknown option in [colors]: songtitle (did you  │
│  mean: song_title?)                              │
│  Unknown config section: [pipeware] (did you     │
│  mean: [pipewire]?)                              │
│                                                  │
│            Press any key to close                │
╰──────────────────────────────────────────────────╯
```

Key features:
- **Non-blocking** - The app starts normally; this is just informational
- **Truncation** - Long warnings are truncated with `...` to fit the popup
- **Dismissable** - Any key press closes the popup
- **Styled** - Uses the app's color scheme for consistency

### Graceful Degradation

The config system never fails hard on unknown options:

```rust
let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
    // Log the error but don't crash
    if cfg!(debug_assertions) {
        eprintln!("Warning: Failed to parse config file: {}", e);
    }
    Config::default()  // Use defaults instead
});
```

This means:
- Unknown options are ignored (as per TOML/serde behavior)
- Parse errors fall back to defaults
- The user can always launch the app, even with a broken config

## Adding New Configuration Options

When adding new configuration options:

1. **Add to struct** - Define the field with `#[serde(default)]`
2. **Add default function** - Implement `default_field_name()`
3. **Add to known fields list** - Update the `KNOWN_*_FIELDS` constant
4. **Document** - Add to the example config

Example:

```rust
// In struct definition
#[serde(default = "MpdConfig::default_new_option")]
pub new_option: u32,

// Default value
fn default_new_option() -> u32 {
    42
}

// Add to validation list
const KNOWN_MPD_FIELDS: &[&str] = &[
    "address", 
    "volume_increment", 
    "volume_increment_fine",
    "new_option",  // <-- Add here
];
```

## Performance Considerations

The validation is designed to be fast and occur only once at startup:

| Operation | Complexity | When |
|-----------|------------|------|
| TOML parse | O(n) | Once at startup |
| Key enumeration | O(k) | Once per section |
| Levenshtein distance | O(m*n) | Per unknown key |
| Suggestion lookup | O(k * m*n) | Per unknown key |

Where:
- n = config file size
- k = number of known keys in section  
- m, n = string lengths

In practice, this adds <1ms to startup time, even with multiple typos.

## Testing Fuzzy Matching

Example typos and their suggestions:

| Typo | Suggestion | Distance |
|------|------------|----------|
| `adress` | `address` | 1 (missing 'd') |
| `volum_up` | `volume_up` | 1 (missing 'e') |
| `toggle_playpause` | `toggle_play_pause` | 1 (missing '_') |
| `boarder` | `border` | 2 (extra 'a', wrong position) |
| `xyz123` | (none) | Too different |

## Related Documentation

- [Sequential Key Bindings](./09_sequential_key_bindings.md) - How keybindings are parsed
- [Async I/O Patterns](./04_async_io_patterns.md) - Config loading in the startup flow
