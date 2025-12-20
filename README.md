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
  -c, --config <CONFIG>        Path to config file
  -a, --address <ADDRESS>      MPD server address (overrides config)
  -m, --music-dir <MUSIC_DIR>  Music directory path (overrides config)
  -h, --help                   Print help
  -V, --version                Print version
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
mode = "fae280"
track_duration = "#e16a7c"
```

</details>

## ⚡ Inspired By

- [rmpc](https://rmpc.mierak.dev/)
- [inori](https://github.com/eshrh/inori)
