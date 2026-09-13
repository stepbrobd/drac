# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { crane, lib, ... }: {
    legacyPackages.crates =
      let
        override = crate:
          let file = ../../crates/${crate}/crane.nix;
          in if lib.pathExists file then import file else { };
      in
      # Force export cargo deps
      assert !(lib.elem "drac-deps" crane.crateDirs);
      { drac-deps = crane.cargoArtifacts; }
      //
      lib.genAttrs
        # Drop crates w/ { disable = true; }
        (lib.filter (crate: !((override crate).disable or false)) crane.crateDirs)
        (crate: crane.builder crate (lib.removeAttrs (override crate) [ "disable" ]));
  };
}
