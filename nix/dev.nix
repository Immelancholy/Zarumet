{
  rustToolchain,
  rustPlatform,
  pipewire,
  pkg-config,
  libclang,
  clang,
}:
let
  zarumetCargoToml = fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = "zarumet";
  version = "${zarumetCargoToml.package.version}-git";
  src = ../.;

  buildType = "debug";

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
