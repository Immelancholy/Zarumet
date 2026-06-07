{
  rustToolchain,
  rustPlatform,
  pipewire,
  pkg-config,
  libclang,
  clang,
}:
let
  zarumetCargoLock = fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = "zarumet";
  version = zarumetCargoLock.package.version;
  src = ../.;

  buildInputs = [
    pipewire
  ];

  nativeBuildInputs = [
    pkg-config
    clang
  ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  cargoLock.lockFile = ../Cargo.lock;

  rustToolchain = rustToolchain;
}
