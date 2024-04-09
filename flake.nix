{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";

  outputs = inputs@{flake-parts, ... }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];  # we can probably add more eventually?
      perSystem = {pkgs, ...}: {
        devShells.default = pkgs.mkShell {
          packages = [pkgs.go];

          shellHook = ''
            # install packages & stuff to current directory
            export GOPATH=$(pwd)/.go
          '';
        };

        # packages.beavercds-ng = pkgs.buildGoModule {
        #   pname = "beavercds-ng";
        #   version = "0.1.0"; # or something

        #   src = pkgs.nix-gitignore.gitignoreSource [] ./.;

        #   vendorHash = pkgs.lib.fakeHash;
        # };
      };
    };
}
