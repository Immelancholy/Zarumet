{
  mkShell,
  rustToolchain,
  rust-analyzer,
  cargo-nextest,
  cargo-about,
  pkg-config,
  pipewire,
  libclang,
  clang,
}:
mkShell {
  nativeBuildInputs = [
    rustToolchain
    rust-analyzer
    cargo-nextest
    cargo-about
    pkg-config
    clang
  ];

  buildInputs = [
    pipewire
  ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  shellHook = /* bash */ ''
    if cargo clippy; then
            nix shell
    fi
  '';
}
