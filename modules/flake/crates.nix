# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { crane, lib, ... }: {
    legacyPackages.crates =
      let
        directories = lib.attrNames (
          lib.filterAttrs
            (_: type: type == "directory")
            (lib.readDir ../../crates));

        override = crate:
          let file = ../../crates/${crate}/crane.nix;
          in if lib.pathExists file then import file else { };
      in
      # Force export cargo deps, i.e. there must NOT be a crate called drac-deps
      { drac-deps = crane.cargoArtifacts; }
      //
      lib.genAttrs
        # Drop crates w/ { disable = true; }
        (lib.filter (crate: !((override crate).disable or false)) directories)
        (crate: crane.builder crate (lib.removeAttrs (override crate) [ "disable" ]));
  };
}
