{
  description = "vesc-rust-poc";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      forSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f (import nixpkgs { inherit system; }));
    in
    {
      devShells = forSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            gcc
            rustc
            rustfmt
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
