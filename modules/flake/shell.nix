# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { crane, pkgs, ... }:
    let
      # Sanitizer flags are nightly only, and the kani package pins one exactly
      fuzz = pkgs.writeShellScriptBin "drac-fuzz" ''
        set -eu
        export PATH=${pkgs.kani.toolchain}/bin:${pkgs.cargo-fuzz}/bin:$PATH
        exec cargo fuzz "$@"
      '';
    in
    {
      devShells.default = crane.lib.devShell {
        packages = with pkgs; [
          # Called by nix fmt, see formatter.nix
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
          fuzz
          kani
        ];
      };
    };
}
