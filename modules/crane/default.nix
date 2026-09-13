# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ inputs }:

let inherit (inputs.nixpkgs) lib; in

pkgs: # pass from call site

lib.fix (crane: {
  toolchain = pkgs.fenix.stable.withComponents [
    "cargo"
    "clippy"
    "rust-analyzer"
    "rust-src"
    "rustc"
    "rustfmt"
  ];

  lib = (inputs.crane.mkLib pkgs).overrideToolchain crane.toolchain;

  src = crane.lib.cleanCargoSource inputs.self.outPath;

  commonArgs = {
    inherit (crane) src;
    strictDeps = true;
    __structuredAttrs = true;

    # Crane cannot read a workspace version, versionOf resolves it below
    pname = "drac";
    version = crane.versionOf "drac";
  };

  # Pre-build/cache deps
  cargoArtifacts = crane.lib.buildDepsOnly crane.commonArgs;

  individualCrateArgs = crane.commonArgs // {
    inherit (crane) cargoArtifacts;
    # Test with cargo-nextest
    doCheck = false;
  };

  fileSetForCrates = crates: lib.fileset.toSource {
    root = ../..;

    fileset = lib.fileset.unions ([
      ../../Cargo.toml
      ../../Cargo.lock
    ] ++ lib.map crane.lib.fileset.commonCargoSources crates
    ++ lib.map (crate: lib.fileset.maybeMissing (crate + "/assets")) crates);
  };

  # crateNameFromCargoToml parses a manifest literally. A crate inheriting
  # version.workspace = true reads back as an attrset and crane falls to 0.0.1
  # Take the literal when there is one and defer to the workspace otherwise
  versionOf = crate:
    let version = (lib.importTOML ../../crates/${crate}/Cargo.toml).package.version or null; in
    if lib.isString version
    then version
    else (lib.importTOML ../../Cargo.toml).workspace.package.version;

  builder = crate: override: crane.lib.buildPackage (
    crane.individualCrateArgs
    //
    {
      pname = crate;
      version = crane.versionOf crate;

      cargoExtraArgs = "--package ${crate}";

      src = crane.fileSetForCrates [ ../../crates/${crate} ];
    }
    //
    override
  );
})
