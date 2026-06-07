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
      overlays = [ (import rust-overlay) ];
      mkPkgs = (system: import nixpkgs { inherit system overlays; });
      forAllSystems = nixpkgs.lib.genAttrs systems;
      mkZarumet =
        package: pkgs:
        pkgs.callPackage ./nix/${package}.nix {
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        };
    in
    {
      formatter = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          config = self.checks.${system}.pre-commit-check.config;
          inherit (config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );

      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
        in
        {
          default = mkZarumet "build" pkgs;
          dev = mkZarumet "dev" pkgs;
          dev-shell = mkZarumet "dev-shell" pkgs;
          zarumet = self.packages.${system}.default;
        }
      );

      overlays = {
        default = final: _prev: {
          zarumet = mkZarumet final;
        };
        zarumet = self.overlays.default;
      };

      checks = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
        in
        {
          pre-commit-check = inputs.git-hooks.lib.${system}.run {
            src = ./.;
            settings.rust = {
              check.cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
            };
            hooks = {
              nixfmt.enable = true;

              rustfmt.enable = true;

              clippy-fix =
                let
                  rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                  clippy = pkgs.writeShellApplication {
                    name = "my-clippy";
                    runtimeInputs = [
                      rust-toolchain
                    ];
                    text = ''
                      export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
                      ${rust-toolchain}/bin/cargo clippy --fix --allow-dirty --offline
                    '';
                  };
                in
                {
                  enable = true;
                  extraPackages = with pkgs; [
                    pipewire
                    pkg-config
                    clang
                    rust-toolchain
                  ];
                  package = clippy;
                  entry = "${pkgs.lib.getExe clippy}";
                  pass_filenames = false;
                };
            };
            package = pkgs.prek;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
        in
        {
          default = pkgs.mkShell {
            buildInputs = enabledPackages;

            nativeBuildInputs = [
              self.packages.${system}.dev-shell
            ];

            shellHook = shellHook + ''
              cargo build
            '';

            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };
        }
      );

      homeModules.default = import ./nix/hm.nix self;
    };
}
