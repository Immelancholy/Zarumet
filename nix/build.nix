{
  lib,
  rustToolchain,
  rustPlatform,
  pkg-config,
  ffmpeg,
  llvmPackages,
  clang,
  glibc,
  libclang,
  stdenv,
  ffmpeg_7,
}: let
  zarumetCargoLock = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
  rustPlatform.buildRustPackage {
    pname = "zarumet";
    version = zarumetCargoLock.package.version;
    src = ../.;

    LIBCLANG_PATH = "${libclang.lib}/lib";
    PKG_CONFIG_PATH = "${ffmpeg_7.dev}/lib/pkgconfig";
    FFMPEG_PKG_CONFIG_PATH = "${ffmpeg_7.dev}/lib/pkgconfig";

    FFMPEG_STATIC = "false";
    FFMPEG_LIBS_DIR = "${ffmpeg_7.out}/lib";
    FFMPEG_INCLUDE_DIR = "${ffmpeg_7.dev}/include";
    VCPKG_ROOT = "";

    # Set proper C compiler flags and override hardcoded paths
    CC = "${stdenv.cc}/bin/cc";
    CXX = "${stdenv.cc}/bin/c++";
    CFLAGS = "-I${ffmpeg_7.dev}/include -I${glibc.dev}/include";
    CXXFLAGS = "-I${ffmpeg_7.dev}/include -I${glibc.dev}/include";
    LDFLAGS = "-L${ffmpeg_7.out}/lib";
    BINDGEN_EXTRA_CLANG_ARGS = "-I${ffmpeg_7.dev}/include -I${glibc.dev}/include -I${stdenv.cc.cc}/include/c++/${stdenv.cc.cc.version} -I${stdenv.cc.cc}/include/c++/${stdenv.cc.cc.version}/${stdenv.hostPlatform.config}";

    nativeBuildInputs = [
      pkg-config
      clang
    ];

    buildInputs = [
      ffmpeg_7
      ffmpeg_7.dev
      libclang
      glibc
      glibc.dev
    ];

    cargoLock.lockFile = ../Cargo.lock;
  }
