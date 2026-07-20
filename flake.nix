{
  description = "vesc-rust-poc";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

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
            extensions = [ "llvm-tools-preview" "rust-src" ];
          };

          devShellPackages = with pkgs; [
            gcc
            pkg-config
            python3
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
            libclang.lib
          ]
          ++ lib.optionals stdenv.isLinux [ dbus.dev ]
          ++ lib.optional (lib.meta.availableOn stdenv.hostPlatform gcc-arm-embedded) gcc-arm-embedded;

          shellHook = ''
            export PATH="$(dirname "$(rustc --print target-libdir)")/bin:$PATH"
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
          '';
        in
        {
          default = pkgs.mkShell {
            packages = devShellPackages;
            inherit shellHook;
          };
        });
    };
}
