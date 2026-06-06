{
  description = "A terminal album art viewer for mpd, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      mkZarumet =
        package: pkgs:
        let
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
        in
        pkgs.callPackage ./nix/${package}.nix {
          rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
        };
    in
    {
      packages = forAllSystems (pkgs: {
        default = mkZarumet "build" pkgs;
        dev = mkZarumet "dev" pkgs;
        zarumet = self.packages.default;
      });
      devShells = forAllSystems (
        pkgs:
        let
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;
          rustToolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          dev = pkgs.callPackage ./nix/shell.nix {
            inherit rustToolchain;
          };
          direnv = pkgs.mkShell {
            packages = [
              self.packages.${pkgs.stdenv.hostPlatform.system}.dev
            ];
          };
        }
      );
      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);
      overlays = {
        default = final: _prev: {
          zarumet = mkZarumet final;
        };
        zarumet = self.overlays.default;
      };

      homeModules.default = import ./nix/hm-module.nix self;
    };
}
