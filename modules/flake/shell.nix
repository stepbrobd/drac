# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { crane, pkgs, ... }: {
    devShells.default = crane.lib.devShell {
      packages = with pkgs; [
        # Formatter stuff
        deno
        nixpkgs-fmt
        taplo

        # cargo   # from crane
        # clippy  # from crane
        # rustc   # from crane
        # rustfmt # from crane
        # rust-analyzer # from crane
        cargo-hakari
        cargo-nextest
      ];
    };
  };
}
