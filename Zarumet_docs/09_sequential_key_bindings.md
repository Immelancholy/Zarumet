# Sequential Key Bindings (Vim-Style)

## The Problem

Single-key bindings have limited namespace:
- 26 letters + modifiers = ~100 combinations
- Many actions compete for intuitive keys
- Vim users expect multi-key sequences like `gg`, `dd`, `ZZ`

## Solution: Sequential Key State Machine

Support multi-key sequences with timeout:

```
User presses 'g' → Start sequence, wait for more input
User presses 'g' again (within timeout) → Execute "go to top"
Timeout expires → Cancel sequence, reset state
```

## State Machine Design

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum KeyState {
    Idle,
    Awaiting {
        sequence: Vec<(KeyModifiers, KeyCode)>,
        timeout: Instant,
    },
}

pub struct KeyBinds {
    // Single-key bindings (fast path)
    global_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    queue_map: HashMap<(KeyModifiers, KeyCode), MPDAction>,
    // ...
    
    // Multi-key sequences
    sequential_bindings: Vec<SequentialKeyBinding>,
    
    // Current state
    current_state: KeyState,
    default_timeout: Duration,
}
```

### Sequential Binding Definition

```rust
#[derive(Debug, Clone)]
pub struct SequentialKeyBinding {
    pub sequence: Vec<(KeyModifiers, KeyCode)>,
    pub action: MPDAction,
}

// Examples:
// "g g" → GoToTop
// "d d" → ClearQueue  
// "shift-z shift-z" → Quit (vim's ZZ)
```

## Key Handling Algorithm

```rust
impl KeyBinds {
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        mode: &MenuMode,
        panel_focus: &PanelFocus,
    ) -> Option<MPDAction> {
        let key_tuple = (key.modifiers, key.code);

        // If already in a sequence, continue it
        if !matches!(self.current_state, KeyState::Idle) {
            return self.handle_sequential_input(key_tuple, mode, panel_focus);
        }

        // Try single-key bindings first (fast path)
        if let Some(action) = self.lookup_single_key(key_tuple, mode, panel_focus) {
            return Some(action);
        }

        // Check if this key could start a sequence
        if self.could_start_sequence(key_tuple) {
            self.current_state = KeyState::Awaiting {
                sequence: vec![key_tuple],
                timeout: Instant::now() + self.default_timeout,
            };
            return None;  // Wait for more input
        }

        None
    }
}
```

### Sequence Continuation

```rust
fn handle_sequential_input(
    &mut self,
    key_tuple: (KeyModifiers, KeyCode),
    mode: &MenuMode,
    panel_focus: &PanelFocus,
) -> Option<MPDAction> {
    match &mut self.current_state {
        KeyState::Idle => None,
        KeyState::Awaiting { sequence, timeout } => {
            // Check timeout
            if *timeout < Instant::now() {
                self.current_state = KeyState::Idle;
                // Retry as new single key
                return self.handle_key(
                    KeyEvent::new(key_tuple.1, key_tuple.0),
                    mode,
                    panel_focus,
                );
            }

            // Add to sequence
            sequence.push(key_tuple);

            // Check for complete match
            for binding in &self.sequential_bindings {
                if binding.sequence == *sequence {
                    self.current_state = KeyState::Idle;
                    return Some(binding.action.clone());
                }
            }

            // Check if sequence could still match something
            let has_prefix_match = self.sequential_bindings.iter().any(|b| {
                sequence.len() <= b.sequence.len() 
                    && b.sequence.starts_with(sequence)
            });

            if has_prefix_match {
                // Refresh timeout and continue waiting
                *timeout = Instant::now() + self.default_timeout;
                None
            } else {
                // No possible match, reset
                self.current_state = KeyState::Idle;
                None
            }
        }
    }
}
```

### Prefix Detection

```rust
fn could_start_sequence(&self, key_tuple: (KeyModifiers, KeyCode)) -> bool {
    self.sequential_bindings
        .iter()
        .any(|binding| binding.sequence.first() == Some(&key_tuple))
}
```

## Timeout Management

Called periodically from the main loop:

```rust
impl KeyBinds {
    pub fn update(&mut self) -> Option<MPDAction> {
        if let KeyState::Awaiting { timeout, .. } = &self.current_state {
            if *timeout < Instant::now() {
                self.current_state = KeyState::Idle;
                // Could return partial match action here if desired
            }
        }
        None
    }
}

// In main loop:
loop {
    self.key_binds.update();  // Check for sequence timeout
    // ... handle events
}
```

## Configuration Parsing

Parse space-separated key strings:

```rust
impl BindsConfig {
    /// Parse "g g" into [(None, 'g'), (None, 'g')]
    pub fn parse_binding_string(&self, binding_str: &str) 
        -> Vec<(KeyModifiers, KeyCode)> 
    {
        binding_str
            .split_whitespace()
            .filter_map(|key_str| self.parse_keybinding(key_str))
            .collect()
    }
    
    /// Parse single key like "shift-z" or "ctrl-c"
    pub fn parse_keybinding(&self, key_str: &str) 
        -> Option<(KeyModifiers, KeyCode)> 
    {
        let parts: Vec<&str> = key_str.split('-').collect();
        
        let mut modifiers = KeyModifiers::NONE;
        let key_part = parts.last()?;
        
        // Parse modifiers
        for part in &parts[..parts.len() - 1] {
            match *part {
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return None,
            }
        }
        
        // Parse key code
        let code = match *key_part {
            "esc" => KeyCode::Esc,
            "enter" => KeyCode::Enter,
            "space" => KeyCode::Char(' '),
            c if c.len() == 1 => {
                let ch = c.chars().next()?;
                if modifiers.contains(KeyModifiers::SHIFT) {
                    KeyCode::Char(ch.to_ascii_uppercase())
                } else {
                    KeyCode::Char(ch)
                }
            }
            _ => return None,
        };
        
        Some((modifiers, code))
    }
}
```

## Building Key Maps

Separate single-key and sequential bindings:

```rust
fn add_enhanced_binding_for_action(
    &self,
    binding_strings: &[String],
    action: MPDAction,
    single_map: &mut HashMap<(KeyModifiers, KeyCode), MPDAction>,
    sequential_bindings: &mut Vec<SequentialKeyBinding>,
) {
    for binding_str in binding_strings {
        let key_sequence = self.parse_binding_string(binding_str);

        if key_sequence.len() == 1 {
            // Single key → fast HashMap lookup
            single_map.insert(key_sequence[0], action.clone());
        } else if key_sequence.len() > 1 {
            // Multi-key → sequential binding
            sequential_bindings.push(SequentialKeyBinding {
                sequence: key_sequence,
                action: action.clone(),
            });
        }
    }
}
```

## Default Vim-Style Bindings

```toml
[binds]
# Single keys
scroll_up = ["k", "up"]
scroll_down = ["j", "down"]
play_selected = ["enter", "l"]

# Sequential keys (space-separated)
go_to_top = ["g g"]           # vim: gg
go_to_bottom = ["shift-g"]    # vim: G
clear_queue = ["d d"]         # vim-like: dd
quit = ["q", "esc", "shift-z shift-z"]  # vim: ZZ
```

## UI Feedback

Show pending sequence to user:

```rust
impl KeyBinds {
    pub fn get_current_sequence(&self) -> Vec<(KeyModifiers, KeyCode)> {
        match &self.current_state {
            KeyState::Awaiting { sequence, .. } => sequence.clone(),
            _ => Vec::new(),
        }
    }

    pub fn is_awaiting_input(&self) -> bool {
        matches!(self.current_state, KeyState::Awaiting { .. })
    }
}

// In status bar:
if key_binds.is_awaiting_input() {
    let seq = key_binds.get_current_sequence();
    render_text(format!("Waiting: {:?}...", seq));
}
```

## Performance Considerations

### Fast Path for Single Keys

```rust
// Single-key lookup: O(1) HashMap
if let Some(action) = self.global_map.get(&key_tuple) {
    return Some(action.clone());
}

// Sequential check: O(n) where n = number of sequential bindings
// Only reached if single-key lookup fails
```

### Memory Usage

```rust
// Single keys: ~50 entries × 24 bytes = ~1.2KB
// Sequential: ~10 entries × 48 bytes = ~480 bytes
// Total: < 2KB for all key bindings
```

## Edge Cases

### 1. Overlapping Prefixes

```rust
// "d" is both single-key (delete) and prefix for "d d" (clear all)
// Solution: Single-key takes priority, "d d" requires second press
```

### 2. Modifier + Sequence

```rust
// "shift-z shift-z" = ZZ in vim
// Each key in sequence can have its own modifiers
```

### 3. Timeout During Rapid Input

```rust
// User types "g" then waits 2 seconds then "g"
// First "g" times out, second "g" starts new sequence
// Result: Nothing happens (sequence reset before completion)
```

## Testing

```rust
#[test]
fn test_sequential_binding() {
    let mut binds = create_test_binds();
    
    // First 'g' - starts sequence
    let result = binds.handle_key(key('g'), &MenuMode::Queue, &PanelFocus::Queue);
    assert_eq!(result, None);
    assert!(binds.is_awaiting_input());
    
    // Second 'g' - completes sequence
    let result = binds.handle_key(key('g'), &MenuMode::Queue, &PanelFocus::Queue);
    assert_eq!(result, Some(MPDAction::GoToTop));
    assert!(!binds.is_awaiting_input());
}

#[test]
fn test_sequence_timeout() {
    let mut binds = create_test_binds();
    binds.default_timeout = Duration::from_millis(100);
    
    // First 'g'
    binds.handle_key(key('g'), &MenuMode::Queue, &PanelFocus::Queue);
    
    // Wait for timeout
    std::thread::sleep(Duration::from_millis(150));
    binds.update();
    
    // Should be back to idle
    assert!(!binds.is_awaiting_input());
}
```

## Related Files

- `src/binds.rs` - KeyBinds state machine
- `src/config.rs` - Binding configuration and parsing
- `src/app/event_handlers.rs` - Integration with event loop
