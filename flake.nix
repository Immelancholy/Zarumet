{
  description = "A terminal album art viewer, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
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
      default = pkgs.callPackages ./nix/shell.nix;
    });
    formatter = forAllSystems (pkgs: pkgs.alejandra);
    overlays.default = final: _prev: {
      zarumet = mkZarumet final;
    };
  };
}
