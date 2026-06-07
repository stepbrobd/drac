{
  outputs = inputs: inputs.autopilot.lib.mkFlake
    {
      inherit inputs;

      autopilot = {
        nixpkgs.instances.pkgs = inputs.nixpkgs;
        parts.path = ./modules/flake;
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
  };
}
