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

      # Declared rather than counted from the source
      # A count read from the file a mutation edits moves with the mutation
      # Adding or removing a proof is meant to be a visible change here
      proofCount = 2;

      # cargo kani runs per package, and a proof in any other crate would
      # verify nowhere, which the check below refuses rather than ignores
      proofCrate = "drac-config";

      # individualCrateArgs turns doCheck off, and a package build would run
      # cargo test rather than nextest, leaving this the only place they run
      testsFor = crate: crane.lib.cargoNextest (crane.commonArgs // {
        inherit (crane) cargoArtifacts;
        pname = crate;
        version = crane.versionOf crate;
        cargoNextestExtraArgs = "--package ${crate}";
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

        # Its own workspace, which leaves every other clippy blind to it
        # A rename in drac-config breaks the target and only this run sees it
        fuzz-lint =
          let
            src = lib.fileset.toSource {
              root = ../..;
              fileset = lib.fileset.unions [
                ../../Cargo.lock
                ../../Cargo.toml
                ../../crates/drac-config
                ../../fuzz
              ];
            };
          in
          pkgs.stdenv.mkDerivation {
            name = "drac-fuzz-lint";
            inherit src;
            nativeBuildInputs = [ crane.toolchain pkgs.rustPlatform.cargoSetupHook ];
            cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ../../fuzz/Cargo.lock;
            };
            cargoRoot = "fuzz";
            buildPhase = ''
              set -u
              cargo clippy --locked --offline --manifest-path fuzz/Cargo.toml \
                --all-targets -- -D warnings
            '';
            installPhase = ''touch "$out"'';
          };

        # Two workspaces resolve separately, and the fuzz target parses with
        # whichever serde_json its own lock picked
        lock-skew =
          let
            versions = lock:
              let packages = (lib.importTOML lock).package; in
              lib.listToAttrs (lib.map
                (name: lib.nameValuePair name
                  (lib.map (p: p.version) (lib.filter (p: p.name == name) packages)))
                (lib.unique (lib.map (p: p.name) packages)));
            root = versions ../../Cargo.lock;
            other = versions ../../fuzz/Cargo.lock;
            # A package the root carries at two versions is not skew, and a
            # version the fuzz lock alone introduces is
            skewed = lib.filter
              (name:
                let carried = root.${name} or [ ]; in
                carried != [ ] && !(lib.all (v: lib.elem v carried) other.${name}))
              (lib.attrNames other);
          in
          pkgs.runCommand "drac-lock-skew"
            { skewed = lib.concatStringsSep " " skewed; } ''
            set -u
            if [ -n "$skewed" ]; then
              echo "The root and fuzz locks disagree on:" >&2
              echo "$skewed" >&2
              exit 1
            fi
            touch "$out"
          '';

        # Every tracked file is ASCII, commit messages excepted, which this cannot see
        # crates/*/proptest-regressions is excluded because Debug renders a
        # shrunk value with a printable non-ASCII scalar left raw
        ascii = pkgs.runCommand "drac-ascii" { } ''
          set -u
          cd ${inputs.self}
          # Redirecting to a file here would write into the read-only store,
          # and the failed redirect would read back as grep finding nothing
          status=0
          offending=$(LC_ALL=C grep -rl --exclude-dir=proptest-regressions \
            "$(printf '[\200-\377]')" .) || status=$?
          # grep answers 1 for no match and 2 for a failure, and only 1 is clean
          if [ "$status" -gt 1 ]; then
            echo "The ASCII scan itself failed with status $status" >&2
            exit 1
          fi
          if [ -n "$offending" ]; then
            echo "Not ASCII:" >&2
            echo "$offending" >&2
            exit 1
          fi
          touch "$out"
        '';

        # cargo kani exits 0 when it finds no harness at all
        # cfg(kani) hides a broken proof from every other check
        # This check defends against both
        kani = crane.lib.mkCargoDerivation (crane.commonArgs // {
          inherit (crane) cargoArtifacts;
          pname = "drac-kani";
          doInstallCargoArtifacts = false;
          nativeBuildInputs = [ pkgs.kani ];
          buildPhaseCargoCommand = ''
            set -u
            status=0
            stray=$(grep -rl "kani::proof" crates --include="*.rs" \
              | grep -v "^crates/${proofCrate}/") || status=$?
            if [ "$status" -gt 1 ]; then
              echo "The proof scan itself failed with status $status" >&2
              exit 1
            fi
            if [ -n "$stray" ]; then
              echo "These carry proofs that cargo kani -p ${proofCrate} never runs:" >&2
              echo "$stray" >&2
              exit 1
            fi

            cargo kani -p ${proofCrate} 2>&1 | tee kani.log
            if ! grep -qF "Complete - ${toString proofCount} successfully verified harnesses, 0 failures" kani.log; then
              echo "Expected ${toString proofCount} verified harnesses and no failure" >&2
              exit 1
            fi

            # An assumption that is contradictory, or merely narrower than
            # the proof claims, verifies vacuously over what it excludes
            # An unsatisfied cover is what tells either apart from a proof
            # A proof declaring no cover reports 0 of 0 and is caught here too
            narrowed=$(awk '/cover properties satisfied/ && ($2 != $4 || $2 == 0)' kani.log)
            if [ -n "$narrowed" ]; then
              echo "A proof reaches less than it claims to cover:" >&2
              echo "$narrowed" >&2
              exit 1
            fi

            covered=$(grep -c "cover properties satisfied" kani.log || true)
            if [ "$covered" != "${toString proofCount}" ]; then
              echo "Expected a cover line per proof, found $covered" >&2
              exit 1
            fi
          '';
          installPhaseCommand = ''touch "$out"'';
        });

        # Cargo skips a binary whose required-features are unsatisfied
        # The package then builds to an empty output with no error
        # The version is not compared against the manifest it is built from
        # That would be the same number twice
        # drac_cli_version_is_calver parses it through the type defining calver
        binaries = pkgs.runCommand "drac-binaries" { } ''
          set -u
          "${built.drac}/bin/drac" version | grep -Eqx "drac [0-9]+\.[0-9]+\.[0-9]+"
          "${built.dracd}/bin/dracd"
          touch "$out"
        '';

        # Under serde_json's arbitrary_precision, as_i64 classifies -0 differently
        # and it flips from refused to accepted, moving the pinned accept set
        # Feature unification can turn it on from any crate in the graph
        arbitrary-precision-absent = crane.lib.mkCargoDerivation (crane.commonArgs // {
          inherit (crane) cargoArtifacts;
          pname = "drac-arbitrary-precision-absent";
          doInstallCargoArtifacts = false;
          buildPhaseCargoCommand = ''
            set -u
            # Without --prefix none every line carries a tree drawing prefix
            # and an anchored pattern matches nothing at all
            # dev edges count, since the accept set is asserted under nextest
            cargo tree --locked --all-features --edges normal,dev --prefix none \
              --format "{p} {f}" > features.txt

            # A listing with no serde_json line proves nothing about its features
            if ! grep -qE "^serde_json v" features.txt; then
              echo "The feature listing carries no serde_json line" >&2
              exit 1
            fi
            if grep -E "^serde_json v[^ ]+ .*arbitrary_precision" features.txt; then
              echo "serde_json resolves with arbitrary_precision" >&2
              echo "That moves which numbers canon accepts" >&2
              exit 1
            fi
          '';
          installPhaseCommand = ''touch "$out"'';
        });

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

        # Every file carries an SPDX header
        # The exceptions are generated or carried verbatim
        # REUSE.toml is the one list of them
        spdx = pkgs.runCommand "drac-spdx" { } ''
          set -u
          cd ${inputs.self}
          missing=$(find . -type f ${lib.concatMapStringsSep " " (path: "! -path ${lib.escapeShellArg "./${path}"}") crane.reuseExceptions} \
            -exec sh -c '${crane.hasSpdx} "$1" || echo "$1"' _ {} \;)
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
