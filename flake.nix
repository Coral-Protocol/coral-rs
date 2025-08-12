{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?rev=8768e989f364993d50fbc32dd5ffa38490bbeb95";
    systems.url = "github:nix-systems/default";

    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    systems,
    nixpkgs,
    ...
  } @ inputs: let
    eachSystem = f:
      nixpkgs.lib.genAttrs (import systems) (
        system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              overlays = [inputs.rust-overlay.overlays.default];
            };
            inherit system;
          }
      );
    rustToolchain = eachSystem ({pkgs, ...}: (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml));
  in {
    devShells = eachSystem ({pkgs, ...}: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          pkg-config
          openssl

          rustToolchain.${pkgs.system}
          rust-analyzer-unwrapped
          cargo
          cargo-hack
          cargo-expand
          bacon
        ];
      };
    });
  };
}
