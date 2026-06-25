{
  description = "vesc-rust-poc";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      forSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f (import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          }));
    in
    {
      devShells = forSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            gcc
            (rust-bin.stable.latest.default.override {
              targets = [ "thumbv7em-none-eabihf" ];
            })
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            gcc-arm-embedded
          ];

          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/target"
          '';
        };
      });
    };
}
