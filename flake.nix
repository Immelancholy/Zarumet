{
  description = "A terminal album art viewer for mpd, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
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
    inherit (self) outputs;
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
    devShells = forAllSystems (pkgs: {
      default = pkgs.callPackage ./nix/shell.nix {
        zarumet = packages.${pkgs.stdenv.hostPlatform.system}.default;
      };
    });
    formatter = forAllSystems (pkgs: pkgs.alejandra);
    overlays.default = final: _prev: {
      zarumet = mkZarumet final;
    };

    homeModules.default = import ./nix/hm_module.nix self;
  };
}
