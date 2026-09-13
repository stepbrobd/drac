# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ lib }:

dir:

let
  inherit (lib)
    attrNames
    filter
    pathExists
    readDir
    ;

  entries = readDir dir;
in
filter
  (
    name:
    entries.${name} == "directory"
      && pathExists (dir + "/${name}/default.nix")
  )
  (attrNames entries)
