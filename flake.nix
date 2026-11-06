{
  description = "Valla, steward of the codebase";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "valla";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          # Valla calls git and curl; carry pinned copies with him.
          postInstall = ''
            wrapProgram $out/bin/valla --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.git pkgs.curl ]}
          '';
          meta = {
            description = "Valla, steward of the codebase";
            license = pkgs.lib.licenses.gpl3Plus;
            mainProgram = "valla";
          };
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.git pkgs.curl ]; };
      });
    };
}
