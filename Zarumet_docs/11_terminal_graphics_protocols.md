# Terminal Graphics Protocols

This document explains how Zarumet displays album cover art in the terminal using ratatui-image and terminal graphics protocols.

## Background

Traditional terminals only display text characters. Modern terminals support various graphics protocols that allow displaying images directly in the terminal:

| Protocol | Terminals | Quality | Method |
|----------|-----------|---------|--------|
| **Kitty** | Kitty, WezTerm | Best | Base64 PNG via escape sequences |
| **iTerm2** | iTerm2, Mintty | Good | Base64 inline images |
| **Sixel** | mlterm, xterm | Medium | Specialized format for terminals |
| **Halfblocks** | Any | Fallback | Unicode block characters |

Zarumet uses the `ratatui-image` crate which automatically detects and uses the best available protocol.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Cover Art Pipeline                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  MPD Server ──album_art()──> Raw Bytes                         │
│                                   │                             │
│                                   ▼                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    image crate                           │   │
│  │  ImageReader::new(Cursor::new(raw_data))                │   │
│  │    .with_guessed_format()  // JPEG, PNG, WebP, etc.     │   │
│  │    .decode()               // -> DynamicImage           │   │
│  └───────────────────────┬─────────────────────────────────┘   │
│                          │                                      │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │               ratatui_image::picker::Picker              │   │
│  │  picker.new_resize_protocol(dyn_img)                     │   │
│  │    -> StatefulProtocol (Kitty/iTerm/Sixel/Halfblock)    │   │
│  └───────────────────────┬─────────────────────────────────┘   │
│                          │                                      │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   StatefulImage widget                   │   │
│  │  frame.render_stateful_widget(image, area, &mut proto)  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation

### Protocol Detection and Initialization

The terminal's graphics capabilities are detected once at startup:

From `src/app/main_loop.rs:111-113`:

```rust
// Set up the image picker and protocol
let mut picker = Picker::from_query_stdio().unwrap();
picker.set_background_color([0, 0, 0, 0]);
```

`Picker::from_query_stdio()`:
1. Sends terminal query escape sequences
2. Parses terminal response to identify capabilities
3. Selects the best available graphics protocol
4. Caches font size for proper image scaling

The transparent background `[0, 0, 0, 0]` ensures the image blends with the terminal background.

### Protocol Wrapper

Zarumet wraps the protocol in a simple struct for easy passing through render functions:

From `src/ui/utils.rs:97-99`:

```rust
pub struct Protocol {
    pub image: Option<ratatui_image::protocol::StatefulProtocol>,
}
```

The `Option` allows graceful handling when:
- No image is loaded yet
- Cover art is still being fetched
- The song has no embedded album art

### Image Loading

Cover art is loaded asynchronously and converted to the appropriate protocol:

From `src/app/main_loop.rs:383-391`:

```rust
protocol.image = data
    .as_ref()
    .and_then(|raw_data| {
        image::ImageReader::new(Cursor::new(raw_data))
            .with_guessed_format()
            .ok()
    })
    .and_then(|reader| reader.decode().ok())
    .map(|dyn_img| picker.new_resize_protocol(dyn_img));
```

The pipeline:
1. **Raw bytes** - Album art data from MPD's `album_art` command
2. **Format detection** - `with_guessed_format()` identifies JPEG, PNG, WebP, etc.
3. **Decoding** - `decode()` converts to `DynamicImage`
4. **Protocol conversion** - `new_resize_protocol()` creates terminal-specific encoding

### Rendering

The image is rendered with proper centering and scaling:

From `src/ui/widgets/image.rs`:

```rust
pub fn render_image_widget(
    frame: &mut ratatui::Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    image_area: Rect,
    skip_render: bool,
) {
    use image::imageops::FilterType;

    // Skip rendering when a popup is showing to avoid conflicts
    if skip_render {
        let placeholder = Paragraph::new("").style(Style::default().dark_gray());
        frame.render_widget(placeholder, placeholder_area);
        return;
    }

    if let Some(ref mut img) = protocol.image {
        // Get image dimensions after resizing for the available area
        let resize = Resize::Scale(Some(FilterType::Lanczos3));
        let img_rect = img.size_for(resize.clone(), image_area);

        // Center the image within the available area
        let centered_area = center_image(img_rect, image_area);

        let image = StatefulImage::default().resize(resize);
        frame.render_stateful_widget(image, centered_area, img);
    } else {
        // Show placeholder when no image
        let placeholder = Paragraph::new("No album art")
            .style(Style::default().dark_gray());
        frame.render_widget(placeholder, placeholder_area);
    }
}
```

Key aspects:
- **Lanczos3 filter** - High-quality downscaling for album art
- **Aspect ratio preservation** - Images are scaled to fit, not stretched
- **Centering** - Images are centered in the available area
- **Placeholder** - "No album art" text when image unavailable

### Image Centering

The centering logic ensures album art is properly positioned:

From `src/ui/utils.rs`:

```rust
pub fn center_image(image_dimensions: Rect, available_area: Rect) -> Rect {
    Rect {
        x: available_area.x + (available_area.width - image_dimensions.width) / 2,
        y: available_area.y + (available_area.height - image_dimensions.height) / 2,
        width: image_dimensions.width,
        height: image_dimensions.height,
    }
}
```

This calculates the offset needed to center an image of arbitrary dimensions within the available terminal area.

## Protocol-Specific Behavior

### Kitty Protocol

The Kitty protocol offers the best quality:
- **Direct PNG transmission** - Lossless quality
- **Placement control** - Precise pixel positioning
- **Unicode placeholder** - Uses Unicode characters as anchors
- **Supports transparency** - Alpha channel preserved

### iTerm2 Protocol

Similar to Kitty but older:
- **Base64 inline images** - Embedded in escape sequences
- **Good compatibility** - Works in several terminal emulators
- **Fixed cell alignment** - Images snap to character cells

### Sixel Protocol

A legacy but widely supported option:
- **256 colors** - Limited palette
- **Raster format** - No vector scaling
- **Wide compatibility** - Works in many older terminals

### Halfblocks Fallback

When no graphics protocol is available:
- Uses Unicode block characters (▀, ▄, █)
- Each "pixel" is half a character cell
- Very limited color accuracy
- Works in any terminal with Unicode support

## Popup Handling

When popups are displayed (like config warnings), image rendering is skipped to avoid conflicts:

```rust
// Skip rendering when a popup is showing to avoid 
// terminal graphics protocol conflicts
if skip_render {
    // Render empty placeholder instead
    let placeholder = Paragraph::new("");
    frame.render_widget(placeholder, placeholder_area);
    return;
}
```

This prevents visual glitches that can occur when terminal graphics overlap with normal text rendering.

## Performance Considerations

### Encoding Cache

`StatefulProtocol` caches the encoded image data:

```rust
if let Some(ref mut img) = protocol.image {
    // Check encoding result (and cache it for next frame)
    img.last_encoding_result();
}
```

This means:
- First render: Full encoding (may take a few ms)
- Subsequent renders: Uses cached encoding
- Re-encode on resize: When terminal size changes

### Memory Usage

Image protocol memory usage:

| Stage | Memory |
|-------|--------|
| Raw MPD data | ~100KB-5MB per image |
| DynamicImage | Width × Height × 4 bytes |
| Encoded protocol | Varies by protocol |
| Cover cache | Up to 50 entries (~50-250MB) |

### Resize Performance

The `Resize::Scale(Some(FilterType::Lanczos3))` option:
- **Quality**: Excellent - best for photographic content
- **Speed**: Slower than bilinear, faster than more advanced filters
- **When applied**: Only when target size differs from source

For very large album art, the resize happens during protocol creation, not during each render.

## Terminal Detection Tips

To check which protocol your terminal supports:

```bash
# Query terminal capabilities
echo -e "\033[?2026\$p"  # Synchronized output
echo -e "\033_Gi=31;q\033\\"  # Kitty graphics
```

Common terminal protocol support:
- **Kitty**: Kitty, WezTerm
- **iTerm2**: iTerm2, Mintty, Tabby
- **Sixel**: mlterm, xterm (with +sixel), foot
- **None**: Basic Linux console, old xterm

## Related Documentation

- [Cover Art Prefetching](./05_cover_art_prefetching.md) - Caching and prefetch strategy
- [Dirty Region Rendering](./03_dirty_region_rendering.md) - When cover art triggers redraws
- [Async I/O Patterns](./04_async_io_patterns.md) - Async cover art loading
