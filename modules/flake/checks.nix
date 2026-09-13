# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }:

{
  perSystem = { config, crane, lib, pkgs, ... }:
    let
      # The built set has already dropped a crate disabled through its crane.nix
      # Filtering by directory also drops the exported cargo artifacts
      built = lib.filterAttrs
        (crate: _: lib.elem crate crane.crateDirs)
        config.legacyPackages.crates;

      # individualCrateArgs turns doCheck off, and a package build would run
      # cargo test rather than nextest, so a crate's tests only run here
      testsFor = crate: crane.lib.cargoNextest (crane.commonArgs // {
        inherit (crane) cargoArtifacts;
        pname = crate;
        version = crane.versionOf crate;
        # A crate that has grown no tests yet is not a failure
        cargoNextestExtraArgs = "--package ${crate} --no-tests=pass";
      });
    in
    {
      checks = {
        clippy = crane.lib.cargoClippy (crane.commonArgs // {
          inherit (crane) cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets --all-features -- -D warnings";
        });

        # The only check that compiles drac the way an embedder takes it
        clippy-without-cli = crane.lib.cargoClippy (crane.commonArgs // {
          inherit (crane) cargoArtifacts;
          pname = "drac-without-cli";
          cargoClippyExtraArgs =
            "--package drac --all-targets --no-default-features -- -D warnings";
        });

        # Compiling without the feature says nothing about the dependency graph
        # Clap can be declared unconditionally and still compile
        clap-absent-without-cli = crane.lib.mkCargoDerivation (crane.commonArgs // {
          inherit (crane) cargoArtifacts;
          pname = "drac-clap-absent-without-cli";
          doInstallCargoArtifacts = false;
          buildPhaseCargoCommand = ''
            set -u

            tree() {
              cargo tree --locked --package drac --edges normal --prefix none \
                --no-default-features "$@"
            }

            # Naming the feature keeps this off whatever the defaults happen to be
            with=with-cli.txt
            without=without-cli.txt

            tree --features cli > "$with"
            if ! grep -q "^clap " "$with"; then
              echo "Clap is not reachable through the cli feature" >&2
              exit 1
            fi

            tree > "$without"
            if grep -q "^clap " "$without"; then
              echo "The drac library depends on clap without the cli feature" >&2
              exit 1
            fi
          '';
          installPhaseCommand = ''touch "$out"'';
        });

        # Cargo skips a binary whose required-features are unsatisfied
        # The package then builds to an empty output with no error
        binaries = pkgs.runCommand "drac-binaries" { } ''
          set -u
          "${built.drac}/bin/drac" version \
            | grep -Fqx "drac ${crane.versionOf "drac"}"
          "${built.dracd}/bin/dracd"
          touch "$out"
        '';

        # The resolver's branches reach manifests this workspace does not carry
        crane-path-deps =
          let
            dirs = [ "drac-config" "other" ];
            shared = {
              drac-config = { path = "crates/drac-config"; };
              renamed = { package = "other"; path = "crates/other"; };
              registry = { version = "1"; };
            };
            deps = manifest: crane.pathDepsFrom { crate = "case"; inherit manifest shared dirs; };
            cases = [
              { name = "member-path"; want = [ "other" ]; got = deps { dependencies.other.path = "../other"; }; }
              { name = "member-rename"; want = [ "other" ]; got = deps { dependencies.alias = { package = "other"; path = "../other"; }; }; }
              { name = "inherited-path"; want = [ "drac-config" ]; got = deps { dependencies.drac-config.workspace = true; }; }
              { name = "inherited-rename"; want = [ "other" ]; got = deps { dependencies.renamed.workspace = true; }; }
              { name = "registry"; want = [ ]; got = deps { dependencies.registry.workspace = true; }; }
              { name = "bare-version"; want = [ ]; got = deps { dependencies.blake3 = "1"; }; }
              { name = "dev-dependency"; want = [ "drac-config" ]; got = deps { dev-dependencies.drac-config.workspace = true; }; }
              { name = "build-dependency"; want = [ "drac-config" ]; got = deps { build-dependencies.drac-config.workspace = true; }; }
              { name = "target-table"; want = [ "drac-config" ]; got = deps { target."cfg(unix)".dependencies.drac-config.workspace = true; }; }
              { name = "target-dev-dependency"; want = [ "drac-config" ]; got = deps { target."cfg(unix)".dev-dependencies.drac-config.workspace = true; }; }
              { name = "two-target-tables"; want = [ "drac-config" "other" ]; got = deps { target = { "cfg(unix)".dependencies.drac-config.workspace = true; "cfg(windows)".dependencies.renamed.workspace = true; }; }; }
              { name = "two-dependencies"; want = [ "drac-config" "other" ]; got = deps { dependencies = { drac-config.workspace = true; renamed.workspace = true; }; }; }
            ];
            graph = { a = [ "b" "d" ]; b = [ "c" ]; c = [ ]; d = [ ]; loop = [ "back" ]; back = [ "loop" ]; };
            closure = crate: lib.sort (x: y: x < y) (crane.closureFrom (name: graph.${name}) crate);
          in
          pkgs.runCommand "drac-crane-path-deps"
            {
              failures = lib.concatStringsSep "\n" (
                let
                  disagreed = lib.filter (case: case.got != case.want) cases;
                  accepted = (builtins.tryEval
                    (lib.deepSeq (deps { dependencies.vendored.path = "../vendored"; }) true)).success;
                  chain = lib.concatStringsSep " " (closure "a");
                  cycle = lib.concatStringsSep " " (closure "loop");
                in
                lib.optional (cases == [ ]) "The case table is empty"
                  ++ lib.optional (disagreed != [ ])
                  "Cases disagreed with what they expect: ${lib.concatMapStringsSep ", "
                    (case: "${case.name} gave [${lib.concatStringsSep " " case.got}]") disagreed}"
                  ++ lib.optional accepted "A path dependency with no crate directory did not throw"
                  ++ lib.optional (chain != "a b c d") "Closure over a chain came back as ${chain}"
                  ++ lib.optional (cycle != "back loop") "Closure over a cycle came back as ${cycle}"
              );
            } ''
            set -u
            if [ -n "$failures" ]; then
              echo "$failures" >&2
              exit 1
            fi
            touch "$out"
          '';

        # Every file carries an SPDX header. The exceptions are generated or
        # carried verbatim and are declared in REUSE.toml, keep the two in step
        spdx = pkgs.runCommand "drac-spdx" { } ''
          set -u
          cd ${inputs.self}
          missing=$(find . -type f \
            ! -path ./Cargo.lock ! -path ./flake.lock ! -path ./license.txt \
            -exec sh -c 'head -5 "$1" | grep -q SPDX-License-Identifier || echo "$1"' _ {} \;)
          if [ -n "$missing" ]; then
            echo "Missing SPDX header:" >&2
            echo "$missing" >&2
            exit 1
          fi
          touch "$out"
        '';
      }
      //
      lib.concatMapAttrs
        (crate: drv: {
          "build-${crate}" = drv;
          "test-${crate}" = testsFor crate;
        })
        built;
    };
}
