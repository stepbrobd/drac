# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{ perSystem = { lib, pkgs, ... }: { _module.args.crane = lib.crane.mkLib pkgs; }; }
