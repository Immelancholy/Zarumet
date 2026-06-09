{
  rustToolchain,
  rustPlatform,
  pipewire,
  pkg-config,
  libclang,
  clang,
  lib,
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

  meta = with lib; {
    description = "A performant rust mpd client with a sleek look!";
    homepage = "https://github.com/Immelancholy/Zarumet";
    license = licenses.mit;
    maintainers = with maintainers; [ Immelancholy ];
    mainProgram = "zarumet";
  };
}
