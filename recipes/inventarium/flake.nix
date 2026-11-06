{
  description = "Inventarium, an inventory tracker kept by Valla";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in {
      # The pinned workshop the sensors run in.
      devShells = forAll (pkgs: {
        default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.curl ]; };
      });

      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "inventarium";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
      });
    };
}
