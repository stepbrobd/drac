# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { crane, lib, pkgs, ... }:
    {
      # Call formatting tools from the repository root
      # Called tools must be put in the dev shell
      formatter = pkgs.writeShellScriptBin "formatter" ''
        set -eoux pipefail

        pushd "$(${lib.getExe pkgs.git} rev-parse --show-toplevel)" > /dev/null
        shopt -s dotglob

        set +x
        # git C-quotes a path with a special character unless asked for NUL
        missing=$(${lib.getExe pkgs.git} ls-files -z --cached --others --exclude-standard \
          | grep -zvxF ${lib.escapeShellArgs (lib.concatMap (p: [ "-e" p ]) crane.reuseExceptions)} \
          | while IFS= read -r -d "" f; do
              ${crane.hasSpdx} "$f" || echo "$f"
            done)
        set -x
        if [ -n "$missing" ]; then
          echo "Missing SPDX header:" >&2
          echo "$missing" >&2
          exit 1
        fi

        cargo clippy --all-features -- -D warnings
        cargo fmt --all
        deno fmt **/*.md **/*.yaml
        nixpkgs-fmt .
        taplo format

        shopt -u dotglob
        popd
      '';
    };
}
