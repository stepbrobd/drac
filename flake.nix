# SPDX-FileCopyrightText: 2026 Yifei Sun
# SPDX-License-Identifier: Apache-2.0

{
  outputs = inputs: inputs.autopilot.lib.mkFlake
    {
      inherit inputs;

      autopilot = {
        parts.path = ./modules/flake;

        lib.path = ./lib;
        lib.extensions = with inputs; [
          autopilot.lib
          parts.lib
          { crane.mkLib = import ./modules/crane { inherit inputs; }; }
        ];

        nixpkgs.overlays = with inputs; [ fenix.overlays.default self.overlays.default ];
        nixpkgs.instances.pkgs = inputs.nixpkgs;
      };
    }
    { systems = import inputs.systems; };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    systems.url = "github:nix-systems/triplet";
    # a
    autopilot.url = "github:stepbrobd/autopilot";
    autopilot.inputs.nixpkgs.follows = "nixpkgs";
    autopilot.inputs.parts.follows = "parts";
    autopilot.inputs.systems.follows = "systems";
    # c
    crane.url = "github:ipetkov/crane";
    # f
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };
}
