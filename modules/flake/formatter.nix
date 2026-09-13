# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  perSystem = { pkgs, ... }: {
    # Find workspace root based on git
    # and call formatting tools (called tools must be put in dev shell)
    formatter = pkgs.writeShellScriptBin "formatter" ''
      set -eoux pipefail
      root="$PWD"
      while [[ ! -f "$root/.git/index" ]]; do
        if [[ "$root" == "/" ]]; then
          exit 1
        fi
        root="$(dirname "$root")"
      done

      pushd "$root" > /dev/null
      shopt -s dotglob

      # checks.spdx gates the same rule on tracked files. This copy sees the
      # working tree, catching a new file before it is ever committed
      missing=$(git ls-files --cached --others --exclude-standard | grep -vE '^(Cargo\.lock|flake\.lock|license\.txt)$' | while read -r f; do
        head -5 "$f" | grep -q SPDX-License-Identifier || echo "$f"
      done)
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
