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
      devShells = forSystems (pkgs:
        let
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "thumbv7em-none-eabihf" ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              dbus.dev
              gcc
              gcc-arm-embedded
              pkg-config
              qt5.qtbase.dev
              qt5.qttools
              qt5.qtquickcontrols2
              qt5.qtserialport
              qt5.qtconnectivity
              qt5.qtpositioning
              qt5.qtgamepad
              qt5.qtserialbus
              rustToolchain
              cargo-nextest
            ];

            shellHook = ''
              export PATH="${pkgs.lib.makeBinPath [ rustToolchain ]}:$PATH"
              export CARGO_TARGET_DIR="$PWD/target"
            '';
          };
        });
    };
}
