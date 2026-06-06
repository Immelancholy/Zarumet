{
  description = "A terminal album art viewer for mpd, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
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
      inherit (self) inputs;
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
      formatter = forAllSystems (
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          config = self.checks.${system}.pre-commit-check.config;
          inherit (config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );

      packages = forAllSystems (pkgs: {
        default = mkZarumet "build" pkgs;
        dev = mkZarumet "dev" pkgs;
        zarumet = self.packages.default;
      });

      overlays = {
        default = final: _prev: {
          zarumet = mkZarumet final;
        };
        zarumet = self.overlays.default;
      };

      checks = forAllSystems (
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
        in
        {
          pre-commit-check = inputs.git-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixfmt.enable = true;

              clippy = {
                enable = true;
                settings.extraArgs = "--fix --allow-dirty";
              };
            };
            package = pkgs.prek;
          };
        }
      );

      devShells = forAllSystems (
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
        in
        {
          default = pkgs.mkShell {
            inherit shellHook;
            buildInputs = enabledPackages;
            packages = [
              self.packages.${pkgs.stdenv.hostPlatform.system}.dev
            ];
          };
        }
      );

      homeModules.default = import ./nix/hm.nix self;
    };
}
