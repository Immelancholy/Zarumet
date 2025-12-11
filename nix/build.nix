{
  rustToolchain,
  rustPlatform,
}: let
  zarumetCargoLock = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
  rustPlatform.buildRustPackage {
    pname = "zarumet";
    version = zarumetCargoLock.package.version;
    src = ../.;

    cargoLock.lockFile = ../Cargo.lock;
  }
