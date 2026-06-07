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
      lib.genAttrs
        # drop crates w/ { disable = true; }
        (lib.filter (crate: !((override crate).disable or false)) directories)
        (crate: crane.builder crate (lib.removeAttrs (override crate) [ "disable" ]));
  };
}
