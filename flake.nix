{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      packages' = self.packages.${system};
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        name = "ls";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };

      packages.ls = packages'.default;

      devShells.default = pkgs.mkShell {
        inputsFrom = [packages'.default];
        packages = [pkgs.rust-analyzer];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
