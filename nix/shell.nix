{
  mkShell,
  rustToolchain,
  rust-analyzer,
  cargo-nextest,
  cargo-about,
  alejandra,
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
    alejandra
    pkg-config
    clang
  ];

  buildInputs = [
    pipewire
  ];

  LIBCLANG_PATH = "${libclang.lib}/lib";

  shellHook = ''
    read -p "Which shell do you use?: " shell

    $shell
    exit
  '';
}
