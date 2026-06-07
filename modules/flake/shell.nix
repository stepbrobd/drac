{
  perSystem = { crane, pkgs, ... }: {
    devShells.default = crane.lib.devShell {
      packages = with pkgs; [
        # formatter stuff
        deno
        nixpkgs-fmt

        # cargo # from crane
        # rustc # from crane
        cargo-hakari
        cargo-nextest
        clippy
        rust-analyzer
      ];
    };
  };
}
