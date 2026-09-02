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
          rustToolchain = pkgs.rust-bin.stable."1.97.1".default.override {
            targets = [ "thumbv7em-none-eabihf" ];
            extensions = [ "llvm-tools-preview" "rust-src" ];
          };

          cargo-llvm-cov = pkgs.cargo-llvm-cov;
          cargo-binutils = pkgs.cargo-binutils;

          cargo-test-changed = pkgs.rustPlatform.buildRustPackage rec {
            pname = "cargo-test-changed";
            version = "0.1.1";

            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-G17zPi8UJTEffkEuZKKlwyiMnqxc9Ki9wfEMl+wr4i4=";
            };

            cargoHash = "sha256-vgP2c5fG0JCvSCMCVAr3bGhmG4JCcCG6lqlwCTy1k20=";
            doCheck = false;

            meta = with pkgs.lib; {
              description = "Run tests only for changed workspace crates";
              homepage = "https://github.com/felixpackard/cargo-test-changed";
              license = licenses.mit;
            };
          };

          devShellPackages = with pkgs; [
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
            cargo-insta
            cargo-test-changed
            cargo-llvm-cov
            cargo-binutils
            cargo-hack
            libclang.lib
          ];

          shellHook = ''
            export PATH="${pkgs.lib.makeBinPath [ rustToolchain cargo-llvm-cov ]}:$PATH"
            export CARGO_TARGET_DIR="$PWD/target"
            host_target="$(rustc -vV | sed -n 's/^host: //p')"
            rust_sysroot="$(rustc --print sysroot)"
            export LLVM_COV="$rust_sysroot/lib/rustlib/$host_target/bin/llvm-cov"
            export LLVM_PROFDATA="$rust_sysroot/lib/rustlib/$host_target/bin/llvm-profdata"
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
          '';
        in
        {
          default = pkgs.mkShell {
            packages = devShellPackages;
            inherit shellHook;
          };
        });

      checks = forSystems (pkgs:
        let
          rustToolchain = pkgs.rust-bin.stable."1.97.1".default.override {
            targets = [ "thumbv7em-none-eabihf" ];
            extensions = [ "llvm-tools-preview" "rust-src" ];
          };

          cargo-llvm-cov = pkgs.cargo-llvm-cov;
          cargo-binutils = pkgs.cargo-binutils;

          cargo-test-changed = pkgs.rustPlatform.buildRustPackage rec {
            pname = "cargo-test-changed";
            version = "0.1.1";

            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-G17zPi8UJTEffkEuZKKlwyiMnqxc9Ki9wfEMl+wr4i4=";
            };

            cargoHash = "sha256-vgP2c5fG0JCvSCMCVAr3bGhmG4JCcCG6lqlwCTy1k20=";
            doCheck = false;

            meta = with pkgs.lib; {
              description = "Run tests only for changed workspace crates";
              homepage = "https://github.com/felixpackard/cargo-test-changed";
              license = licenses.mit;
            };
          };
        in
        let checkEnv = ''
          cd $src
          export PATH="${pkgs.lib.makeBinPath [ rustToolchain cargo-llvm-cov ]}:$PATH"
          export CARGO_TARGET_DIR="$TMPDIR/cargo-target"
          host_target="$(rustc -vV | sed -n 's/^host: //p')"
          rust_sysroot="$(rustc --print sysroot)"
          export LLVM_COV="$rust_sysroot/lib/rustlib/$host_target/bin/llvm-cov"
          export LLVM_PROFDATA="$rust_sysroot/lib/rustlib/$host_target/bin/llvm-profdata"
          export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
        '';
        in
        {
          check = pkgs.runCommand "vesc-rust-poc-check" {
            src = ./.;
            nativeBuildInputs = with pkgs; [
              dbus.dev
              gcc
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
              cargo-insta
              cargo-test-changed
              cargo-llvm-cov
              cargo-binutils
              libclang.lib
            ];
          } ''
            ${checkEnv}
            make check
            touch $out
          '';

          check-full = pkgs.runCommand "vesc-rust-poc-check-full" {
            src = ./.;
            nativeBuildInputs = with pkgs; [
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
              cargo-insta
              cargo-test-changed
              cargo-llvm-cov
              cargo-binutils
              libclang.lib
            ];
          } ''
            ${checkEnv}
            make check-full
            touch $out
          '';
        });
    };
}
