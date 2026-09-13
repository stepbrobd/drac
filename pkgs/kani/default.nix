# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ cbmc
, cvc5
, fenix
, fetchFromGitHub
, fetchzip
, kissat
, lib
, makeWrapper
, runCommand
, rustPlatform
, stdenv
, z3
, zlib
}:

let
  # Upstream's manifest still reads 0.67.0 past the tag of that name
  version = "0.67.0-unstable-2026-09-01";
  rev = "b07abe8a72f8eb1ef1ea3521ca6b00a973d341bc";

  # Kani is a rustc driver and its rust-toolchain.toml pins this nightly exactly
  # One definition, because kani-driver loads the toolchain the file below names
  toolchainDate = "2026-08-21";
  channel = "nightly-${toolchainDate}";

  toolchain = (fenix.toolchainOf {
    date = toolchainDate;
    channel = "nightly";
    sha256 = "sha256-PDDMZVp1SdCzABXNAy+Unocj2lrQOfZy0EUgu66k520=";
  }).withComponents [
    "cargo"
    "llvm-tools"
    "rust-src"
    "rustc"
    "rustc-dev"
    "rustfmt"
  ];

  src = fetchzip {
    url = "https://github.com/model-checking/kani/archive/${rev}.tar.gz";
    hash = "sha256-BbUwq4rCzsCWSKFCkUWXyjUex3ANaHruQdqQPuiehog=";
  };

  # Cargo refuses to resolve the workspace without this optional dependency
  charon = fetchFromGitHub {
    owner = "AeneasVerif";
    repo = "charon";
    rev = "b250680abd40ff1aaa07081d0497dc2755ed112e";
    hash = "sha256-J/of5Bdj3fUFud7XPxVZXNtdUx1rMDiqS8fsptQVfQg=";
  };

  kaniVendor = rustPlatform.fetchCargoVendor {
    inherit src;
    name = "kani-${version}";
    hash = "sha256-2R6pWBzrvtH4yqXKAS7MlH0lmbZxXXgHVFaSKiev3/M=";
  };

  # kani-compiler's build script rpaths librustc_driver through a rustup layout
  rustupHome = runCommand "kani-${version}-rustup" { } ''
    mkdir -p $out/toolchains
    ln -s ${toolchain} $out/toolchains/${channel}
  '';

  # kani-driver calls cbmc and the solvers by bare name at verification time
  solvers = [ cbmc cvc5 kissat z3 ];
in
stdenv.mkDerivation {
  pname = "kani";
  inherit src version;

  nativeBuildInputs = [ makeWrapper toolchain ];
  # librustc_driver pulls zlib in through the rustc internals kani-compiler links
  buildInputs = [ zlib ];

  # rlibs carry the MIR Kani verifies against, and stripping throws it away
  dontStrip = true;
  # Shrinking the rpath would drop the toolchain that kani-compiler loads
  dontPatchELF = true;

  postPatch = ''
    # The pin above and upstream's own rust-toolchain.toml are two definitions
    # A rev bump that moves upstream's would build against one nightly while
    # $out/rust-toolchain-version names another, which kani-driver then loads
    grep -qF 'channel = "${channel}"' rust-toolchain.toml

    cp -r ${charon}/. charon/
    chmod -R u+w charon
  '';

  configurePhase = ''
    runHook preConfigure

    export CARGO_HOME=$NIX_BUILD_TOP/cargo-home
    export RUSTUP_HOME=${rustupHome}
    export RUSTUP_TOOLCHAIN=${channel}

    # The sysroot step rebuilds std, whose locked dependencies ship with rust-src
    # One directory source, because Cargo replaces crates-io with a single one
    vendor=$NIX_BUILD_TOP/vendor
    cp -r ${kaniVendor} $vendor
    chmod -R u+w $vendor
    # source-registry-0 is the directory name fetchCargoVendor writes
    cp -rn ${toolchain}/lib/rustlib/src/rust/library/vendor/. $vendor/source-registry-0/
    substitute $vendor/.cargo/config.toml vendor.toml --subst-var-by vendor $vendor
    cat vendor.toml >> .cargo/config.toml
    rm vendor.toml

    runHook postConfigure
  '';

  buildPhase = ''
    runHook preBuild

    cargo build-dev --release
    cargo build --release -p kani-cov

    runHook postBuild
  '';

  # The release layout kani-driver expects, carrying the toolchain that
  # cargo kani setup would otherwise download
  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin $out/library $out/libexec
    cp target/kani/bin/kani-compiler target/kani/bin/kani-driver $out/bin/
    cp target/release/kani-cov $out/bin/
    cp -r target/kani/lib target/kani/no_core target/kani/playback $out/
    cp -r library/kani library/kani_macros library/std $out/library/
    ln -s ${toolchain} $out/toolchain
    echo ${channel} > $out/rust-toolchain-version
    rustc --version > $out/rustc-version

    # goto-cc preprocesses through a compiler it looks up by the name gcc
    # Linked on every platform, since the wrapper is gcc on Linux anyway
    ln -s ${stdenv.cc}/bin/cc $out/libexec/gcc

    for bin in cargo-kani kani; do
      makeWrapper $out/bin/kani-driver $out/bin/$bin \
        --argv0 $bin \
        --prefix PATH : "$out/libexec:${lib.makeBinPath solvers}"
    done

    runHook postInstall
  '';

  # The dev shell's fuzz wrapper needs a nightly too, and reading it from here
  # keeps the pin where the decision that fixes it lives
  passthru = { inherit toolchain; };

  meta = {
    description = "Bit-precise model checker for Rust";
    homepage = "https://github.com/model-checking/kani";
    license = with lib.licenses; [ asl20 mit ];
    mainProgram = "kani";
    platforms = [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
  };
}
