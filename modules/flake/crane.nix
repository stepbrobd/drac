{ inputs, self, ... }:

{
  perSystem = { crane, lib, pkgs, ... }: {
    _module.args.crane = {
      lib = inputs.crane.mkLib pkgs;

      src = crane.lib.cleanCargoSource self.outPath;

      commonArgs = {
        inherit (crane) src;
        strictDeps = true;
        __structuredAttrs = true;

        # crane cant infer pname/version
        # set a placeholder and override in per crate drv
        pname = "drac";
        version = "2001.717.0";
      };

      # pre-build/cache deps
      cargoArtifacts = crane.lib.buildDepsOnly crane.commonArgs;

      individualCrateArgs = crane.commonArgs // {
        inherit (crane) cargoArtifacts;
        # test with cargo-nextest
        doCheck = false;
      };

      fileSetForCrates = crates: lib.fileset.toSource {
        root = ../..;

        fileset = lib.fileset.unions ([
          ../../Cargo.toml
          ../../Cargo.lock
        ] ++ lib.map crane.lib.fileset.commonCargoSources crates);
      };
    };
  };
}
