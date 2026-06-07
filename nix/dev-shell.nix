{
  rustToolchain,
  writeShellApplication,
}:
writeShellApplication {
  name = "zarumet";
  runtimeInputs = [
    rustToolchain
  ];
  text = ''
    cargo run
  '';
}
