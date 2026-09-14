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

  # One definition of the header predicate, read by checks.spdx and by nix fmt
  # The tools are named, since nix fmt runs with whatever PATH the caller has
  hasSpdx = lib.getExe (pkgs.writeShellApplication {
    name = "has-spdx";
    runtimeInputs = [ pkgs.coreutils pkgs.gnugrep ];
    text = ''head -5 "$1" | grep -q SPDX-License-Identifier'';
  });

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
    ]
    ++ lib.map crane.lib.fileset.commonCargoSources crates
    ++ lib.map (crate: lib.fileset.maybeMissing (crate + "/assets")) crates);
  };

  # crateNameFromCargoToml parses a manifest literally. A crate inheriting
  # version.workspace = true reads back as an attrset and crane falls to 0.0.1
  # Take the literal when there is one and defer to the workspace otherwise
  versionOf = crate:
    let version = (lib.importTOML ../../crates/${crate}/Cargo.toml).package.version or null; in
    if lib.isString version
    then version
    else (lib.importTOML ../../Cargo.toml).workspace.package.version
      or (throw "Crate ${crate} has no version and the workspace declares none");

  # REUSE.toml is the one list of files that cannot carry an SPDX header, and
  # both the check and the formatter read it from here
  # A glob would reach find and grep -vxF differently, and the two consumers
  # have to agree, so refuse one rather than let them disagree
  reuseExceptions =
    let
      paths = lib.concatMap
        (annotation: lib.toList annotation.path)
        (lib.importTOML ../../REUSE.toml).annotations;
      globbed = lib.filter (lib.hasInfix "*") paths;
    in
    if globbed == [ ]
    then paths
    else throw "REUSE.toml globs are read differently by each consumer: ${lib.concatStringsSep " " globbed}";

  crateDirs = lib.attrNames (
    lib.filterAttrs (_: type: type == "directory") (lib.readDir ../../crates));

  # Split from pathDepsOf, which lets a check drive it with literal manifests
  pathDepsFrom = { crate, manifest, shared, dirs }:
    let
      depsOf = table: (table.dependencies or { })
        // (table.dev-dependencies or { })
        // (table.build-dependencies or { });

      # Cargo resolves every target table, not only the host's
      declared = lib.foldl'
        (acc: table: acc // depsOf table)
        (depsOf manifest)
        (lib.attrValues (manifest.target or { }));

      # An inherited dependency keeps its path and rename in the workspace table
      # Cargo ignores both on the member, leaving that table the only source
      resolve = name: spec:
        if spec.workspace or false
        then shared.${name} or { }
        else spec;

      # Only a crate under crates/ is reachable from the fileset below
      dirOf = name: spec:
        let resolved = resolve name spec; in
        if !(resolved ? path) then null
        else
          let package = resolved.package or name; in
          if lib.elem package dirs
          then package
          else throw "Path dependency ${package} of ${crate} has no directory under crates/";
    in
    lib.filter (dep: dep != null) (lib.mapAttrsToList dirOf declared);

  # Cargo loads the manifest of every path dependency during a build
  # The source for one crate therefore carries its whole closure
  pathDepsOf = crate: crane.pathDepsFrom {
    inherit crate;
    manifest = lib.importTOML ../../crates/${crate}/Cargo.toml;
    shared = (lib.importTOML ../../Cargo.toml).workspace.dependencies or { };
    dirs = crane.crateDirs;
  };

  closureFrom = depsOf: crate: lib.map (entry: entry.key) (builtins.genericClosure {
    startSet = [{ key = crate; }];
    operator = entry: lib.map (dep: { key = dep; }) (depsOf entry.key);
  });

  pathDepClosureOf = crane.closureFrom crane.pathDepsOf;

  builder = crate: override: crane.lib.buildPackage (
    crane.individualCrateArgs
    //
    {
      pname = crate;
      version = crane.versionOf crate;

      cargoExtraArgs = "--package ${crate}";

      src = crane.fileSetForCrates
        (lib.map (dep: ../../crates/${dep}) (crane.pathDepClosureOf crate));
    }
    //
    override
  );
})
