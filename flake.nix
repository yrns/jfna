{
  description = "hello, rust";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";
  inputs.utils.url = "github:numtide/flake-utils";
  inputs.rustify.url = "github:yrns/rustify";

  outputs = { self, nixpkgs, utils, rustify }:
    # Cargo.lock is only valid for the current system.
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        crateOverrides = rustify.lib.crateOverrides {
          lockFile = ./Cargo.lock;
          inherit pkgs;
        };
      in
      {
        devShell = pkgs.mkShell {
          inputsFrom = [ crateOverrides ];
        };
      });
}
