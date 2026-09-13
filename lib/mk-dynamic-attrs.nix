# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ lib }:

# mkDynamicAttrs args
{ dir, fun }:

let
  inherit (lib) attrNames filter genAttrs readDir;

  entries = readDir dir;

  # Filter stray files (readme.md, .DS_Store, etc.)
  dirs = filter (name: entries.${name} == "directory") (attrNames entries);
in
genAttrs dirs fun
