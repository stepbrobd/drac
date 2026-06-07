{ inputs, ... }:

{
  perSystem = { crane, pkgs, ... }: {
    _module.args.crane = {
      lib = inputs.crane.mkLib pkgs;

      src = crane.lib.cleanCargoSource inputs.self.outPath;

      # TODO: add common crate args here?
    };
  };
}
