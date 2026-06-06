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

  buildType = "debug";

  nativeBuildInputs = [
    pkg-config
    clang
  ];
  buildInputs = [ pipewire ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  cargoLock.lockFile = ../Cargo.lock;
}
