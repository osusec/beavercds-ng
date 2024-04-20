{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"]; # we can probably add more eventually?
      perSystem = {
        self',
        pkgs,
        system,
        ...
      }: {
        # apply fenix overlay
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.fenix.overlays.default
          ];
        };

        # shell with just rust & stuff for now
        devShells.default = let
          inherit (pkgs) mkShell fenix;
          rust = fenix.stable.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
            "rust-analyzer"
          ];
        in
          mkShell {
            packages = [rust];
          };

        packages.beavercds-ng = pkgs.rustPlatform.buildRustPackage {
          pname = "beavercds-ng";
          version = "0.1.0";

          src = pkgs.nix-gitignore.gitignoreSource [] ./.;

          # probably don't fill in unless testing
          cargoHash = "";
        };

        # yay checks :)
        checks.beavercds-ng = self'.packages.beavercds-ng;

        formatter = pkgs.alejandra;
      };
    };
}
