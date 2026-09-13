# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { lib, pkgs, ... }: {
    # Call formatting tools from the repository root
    # Called tools must be put in the dev shell
    formatter = pkgs.writeShellScriptBin "formatter" ''
      set -eoux pipefail

      pushd "$(${lib.getExe pkgs.git} rev-parse --show-toplevel)" > /dev/null
      shopt -s dotglob

      set +x
      missing=$(${lib.getExe pkgs.git} ls-files --cached --others --exclude-standard \
        | grep -vE '^(Cargo\.lock|flake\.lock|license\.txt)$' \
        | while read -r f; do
            head -5 "$f" | grep -q SPDX-License-Identifier || echo "$f"
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
