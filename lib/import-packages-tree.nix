# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ lib }:

{ dir
, currentFinal
, currentPrev
, inheritedArgs ? { }
}:

let
  inherit (lib)
    callPackageWith
    childDirsWithDefault
    importPackagesTree
    isAttrs
    isDerivation
    isFunction
    length
    makeScope
    mkDynamicAttrs
    pathExists
    tryEval
    ;

  # Expose top-level package args (e.g. fetchgit) shadowed by scope args (e.g. mkDerivation) inherited args always win
  # lib, inputs, pkgsFinal/pkgsPrev and ancestor <scope>Final/<scope>Prev are reserved names that resolve to the flake-level values even if a scope exposes a same-named member
  callArgs = (inheritedArgs.pkgsFinal or currentFinal) // currentFinal // inheritedArgs;
in
mkDynamicAttrs {
  inherit dir;
  fun =
    name:
    let
      pkg = dir + "/${name}";
      hasDefaultNix = pathExists (pkg + "/default.nix");
      childScopeNames = if !hasDefaultNix then childDirsWithDefault pkg else [ ];
      hasChildScopes = length childScopeNames > 0;

      # Lazy eval, only touch currentPrev when the local package path has no default.nix and guard alias throws with tryEval
      hasScopeAttr = !hasDefaultNix && currentPrev ? ${name};
      scopeEval =
        if hasScopeAttr then
          tryEval currentPrev.${name}
        else
          {
            success = false;
            value = null;
          };
      hasScopeValue = scopeEval.success;
      scopeValue = if hasScopeValue then scopeEval.value else null;
      hasOverrideScope = hasScopeValue && isAttrs scopeValue && scopeValue ? overrideScope;
      hasExtend = hasScopeValue && isAttrs scopeValue && scopeValue ? extend;

      # Note that some scopes expose extend rather than overrideScope, e.g. haskellPackages
      # Duck-typed, any prev attrset with overrideScope/extend matches
      # Do not name a local dir after an extensible non-package-scope attr
      # (e.g. pkgs/lib would extend lib itself)
      isScope = hasScopeValue && isAttrs scopeValue && !isDerivation scopeValue && (hasOverrideScope || hasExtend);
      scopeOverride =
        if hasOverrideScope then
          scopeValue.overrideScope
        else
          scopeValue.extend;
    in
    if hasDefaultNix then
      let
        imported = import pkg;
      in
      # Case 1: local package/default.nix always wins over currentPrev attrs
      if !isFunction imported then
        imported
      else
        callPackageWith callArgs pkg { }
    # Case 2: the imported dir is an existing scope in currentPrev
    # I've decided that having a entry point for scoped pkgs to override arguments used is a antipattern
    # Injecting root level pkgsPrev and pkgsFinal with scope level fixedpoints is a better idea
    else if isScope then
      scopeOverride
        (
          scopeFinal: scopePrev:
          # Recurse:
          importPackagesTree {
            dir = pkg;
            currentFinal = scopeFinal;
            currentPrev = scopePrev;
            inheritedArgs = inheritedArgs // {
              "${name}Final" = scopeFinal;
              "${name}Prev" = scopePrev;
            };
          }
        )
    # Case 3: local scope (no default.nix, but has child dirs with default.nix)
    else if hasChildScopes then
      makeScope callPackageWith
        (
          localScopeFinal:
          # Recurse:
          importPackagesTree {
            dir = pkg;
            currentFinal = localScopeFinal;
            # A fresh local scope has nothing to override at this level
            # Outer prev must not leak in or child names matching root attrs misroute to case 2
            currentPrev = { };
            inheritedArgs = inheritedArgs // {
              "${name}Final" = localScopeFinal;
              "${name}Prev" = localScopeFinal;
            };
          }
        )
    # Bail if not scope and does not have default.nix
    else
      throw "Path ${toString pkg} has no default.nix and is not a scope";
}
