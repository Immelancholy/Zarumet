# 🦀 Zarumet 🦀

## Introduction

Zarumet is an mpd client for the terminal written in Rust.

![Zarumet Screenshot](/assets/screenshots/zarumet.png?raw=true)

![Zarumet Screenshot 2](/assets/screenshots/zarumet_2.png?raw=true)

## Dependencies

- [mpd](https://musicpd.org)
- A NerdFont
- A terminal emulator that supports truecolor and UTF-8 encoding.

## 🦀 Installation

```Bash
cargo install zarumet
```

## ❄️ NixOS installation via flake

<details>
<summary>Click to expand NixOS installation instructions</summary>

Add the zarumet repo as a flake input:

```Nix
{
   inputs = {
       zarumet = {
           url = "github:Immelancholy/zarumet";
           inputs.nixpkgs.follows = "nixpkgs";
       };
   };
}
```

Add the zarumet package to your system configuration:

```Nix
{pkgs, inputs, ...}:
{
    environment.systemPackages = with pkgs; [
        inputs.zarumet.packages.${pkgs.stdenv.hostPlatform.system}.default
    ];
}
```

</details>

<details>
<summary>Click to expand NixOS installation via flakes and home manager</summary>

Add the zarumet repo as a flake input:

```Nix
{
   inputs = {
       zarumet = {
           url = "github:Immelancholy/zarumet";
           inputs.nixpkgs.follows = "nixpkgs";
       };
   };
}
```

Add the zarumet module to your homemanager config:

```Nix
  home-manager.users.your-user = {
    imports = [
      inputs.zarumet.homeModules.default
    ];
  };
```

Enable the zarumet program and configure to your liking:

```Nix
{
  programs.zarumet = {
    enable = true;
    settings = {
      mpd = {
        address = "localhost:6600";
      };
      colors = {
        album = "#fae280";
        artist = "#fae280";
        border = "#fae280";
        status = "#fae280";
        title = "#fae280";
      };
      pipewire = {
        bit_perfect_enabled = false;
      };
    };
  };
}
```

</details>

## Usage

```
A TUI MPD client with album art

Usage: zarumet [OPTIONS]

Options:
  -c, --config <CONFIG>    Path to config file
  -a, --address <ADDRESS>  MPD server address (overrides config)
  -h, --help               Print help
  -V, --version            Print version
```

## Configuration

Configuration is stored in a TOML file. The default location is:
`~/.config/zarumet/config.toml`.
You can override this with the `-c` flag.

<details>
<summary>Default Configuration: </summary>

```Toml
[mpd]
address = "localhost:6600"
volume_increment = 5
volume_increment_fine = 1

[colors]
border = "#fae280"
song_title = "#fae280"
album = "#26a0a1"
artist = "#d67751"
border_title = "#8193af"
progress_filled = "#26a0a1"
progress_empty = "#1b1d0e"
paused = "#e16a7c"
playing = "#e16a7c"
stopped = "#e16a7c"
time_separator = "#e16a7c"
time_duration = "#e16a7c"
time_elapsed = "#e16a7c"
queue_selected_highlight = "#b18a4a"
queue_selected_text = "#1b1d0e"
queue_album = "#26a0a1"
queue_song_title = "#fae280"
queue_artist = "#d67751"
queue_position = "#e16a7c"
queue_duration = "#e16a7c"
top_accent = "#e16a7c"
volume = "#26a0a1"
volume_empty = "#1b1d0e"
mode = "#fae280"
track_duration = "#e16a7c"

[binds]
next = [
    ">",
    "shift-j",
    "shift-down",
]
previous = [
    "<",
    "shift-k",
    "shift-up",
]
toggle_play_pause = [
    "space",
    "p",
]
volume_up = ["="]
volume_up_fine = ["+"]
volume_down = ["-"]
volume_down_fine = ["_"]
toggle_mute = ["m"]
cycle_mode_right = [
    "ctrl-l",
    "ctrl-right",
]
cycle_mode_left = [
    "ctrl-h",
    "ctrl-left",
]
clear_queue = ["d d"]
repeat = ["r"]
random = ["z"]
single = ["s"]
consume = ["c"]
quit = [
    "esc",
    "q",
    "ctrl-c",
    "shift-z shift-z",
]
refresh = ["u"]
switch_to_queue_menu = ["1"]
switch_to_artists = ["2"]
switch_to_albums = ["3"]
seek_forward = [
    "shift-l",
    "shift-right",
]
seek_backward = [
    "shift-h",
    "shift-left",
]
scroll_up = [
    "k",
    "up",
]
scroll_down = [
    "j",
    "down",
]
play_selected = [
    "enter",
    "l",
    "right",
]
remove_from_queue = [
    "x",
    "backspace",
    "d d",
]
move_up_in_queue = [
    "ctrl-k",
    "ctrl-up",
]
move_down_in_queue = [
    "ctrl-j",
    "ctrl-down",
]
switch_panel_left = [
    "h",
    "left",
]
switch_panel_right = [
    "l",
    "right",
]
toggle_album_expansion = [
    "l",
    "right",
]
add_to_queue = [
    "a",
    "enter",
]
scroll_up_big = ["ctrl-u"]
scroll_down_big = ["ctrl-d"]
go_to_top = ["g g"]
go_to_bottom = ["shift-g"]
toggle_bit_perfect = ["b"]

[pipewire]
bit_perfect_enabled = false

[logging]
enabled = true
level = "info"
log_to_console = false
append_to_file = true
rotate_logs = true
rotation_size_mb = 10
keep_log_files = 5
```

</details>

<details>
<summary>Bit-Perfect Mode (PipeWire)</summary>

Zarumet supports bit-perfect audio playback by automatically adjusting PipeWire sample rate to match the current song's sample rate. This prevents resampling and ensures the highest quality audio output.

### Requirements

- PipeWire audio server
- MPD configured to output through PipeWire

### Configuration

To enable bit-perfect mode, add a `[pipewire]` section to your config file:

```Toml
[pipewire]
bit_perfect_enabled = true
```

When enabled, Zarumet will automatically discover the supported sample rates from your PipeWire audio system and hardware, eliminating the need for manual configuration. The system will:

1. Query PipeWire for available sample rates from your audio devices
2. Fall back to common sample rates if automatic detection fails
3. Dynamically select the optimal rate for each song

### Usage

Once enabled in the configuration, toggle bit-perfect mode on/off with `b` (default keybind). The mode indicator will appear in the UI when enabled.

When a song plays, Zarumet will automatically:

1. Use the song's native sample rate if it's supported by your audio hardware
2. Otherwise, select the highest compatible supported rate
3. Fall back to 44100 Hz if no compatible rate is found

The system supports common sample rates including:

- `44100` - CD quality
- `48000` - DVD/common digital audio
- `88200` - 2x CD quality
- `96000` - High-resolution audio
- `176400` - 4x CD quality  
- `192000` - High-resolution audio
- `352800` - DXD quality
- `384000` - Ultra high-resolution audio

### CLI Option

You can also set the initial bit-perfect state via the command line:

```Bash
zarumet --bit-perfect on   # Start with bit-perfect enabled
zarumet --bit-perfect off  # Start with bit-perfect disabled
```

The state is persisted between sessions.
</details>

## ⚡ Inspired By

- [rmpc](https://rmpc.mierak.dev/)
- [inori](https://github.com/eshrh/inori)
