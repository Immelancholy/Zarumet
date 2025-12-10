{
  mkShell,
  # zarumet,
  rust-analyzer,
  rustup,
  cargo-nextest,
  cargo-generate,
  alejandra,
  ...
}:
# (mkShell.override {inherit (zarumet) stdenv;}) {
# inputsFrom = [zarumet];
mkShell {
  packages = [
    rust-analyzer
    rustup
    cargo-nextest
    alejandra
    cargo-generate
  ];
}
