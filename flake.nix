{
  description = "A terminal album art viewer for mpd, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    mkZarumet = pkgs: let
      rustBin = rust-overlay.lib.mkRustBin {} pkgs;
    in
      pkgs.callPackage ./nix/build.nix {
        rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
      };
  in rec {
    packages = forAllSystems (pkgs: {
      default = mkZarumet pkgs;
    });
    devShells = forAllSystems (pkgs: let
      rustBin = rust-overlay.lib.mkRustBin {} pkgs;
      rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
    in {
      default = pkgs.callPackage ./nix/shell.nix {
        inherit rustToolchain;
      };
    });
    formatter = forAllSystems (pkgs: pkgs.alejandra);
    overlays.default = final: _prev: {
      zarumet = mkZarumet final;
    };

    homeModules.default = import ./nix/hm-module.nix self;
  };
}
