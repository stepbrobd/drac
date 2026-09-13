# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ inputs, ... }:

{
  perSystem = { crane, lib, pkgs, ... }:
    let
      crates = lib.attrNames (
        lib.filterAttrs (_: type: type == "directory") (lib.readDir ../../crates));

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

        # Every file carries an SPDX header. The exceptions are generated or
        # carried verbatim and are declared in REUSE.toml, keep the two in step
        spdx = pkgs.runCommand "drac-spdx" { } ''
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
      lib.listToAttrs (lib.map (crate: lib.nameValuePair "test-${crate}" (testsFor crate)) crates);
    };
}
