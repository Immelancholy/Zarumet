# 🦀 Zarumet 🦀

## Introduction

Zarumet is an mpd album art viewer for the terminal written in Rust.

![Zarumet Screenshot](/assets/screenshots/zarumet.png?raw=true)

## Dependencies

- [mpd](https://musicpd.org)
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
        music_dir = "/home/youruser/Music";
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

Default configuration:

```Toml
[mpd]
address = "localhost:6600"
music_dir = "/home/youruser/Music"

[colors]
album = "#fae280"
artist = "#fae280"
border = "#fae280"
status = "#fae280"
title = "#fae280"
```
