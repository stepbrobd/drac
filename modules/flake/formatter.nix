{
  perSystem = { lib, pkgs, ... }: {
    formatter = pkgs.writeShellScriptBin "formatter" ''
      set -eoux pipefail
      shopt -s globstar
      root="$PWD"
      while [[ ! -f "$root/.git/index" ]]; do
        if [[ "$root" == "/" ]]; then
          exit 1
        fi
        root="$(dirname "$root")"
      done
      pushd "$root" > /dev/null
      ${lib.getExe pkgs.nixpkgs-fmt} .
      popd
    '';
  };
}
