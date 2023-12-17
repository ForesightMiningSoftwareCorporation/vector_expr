{
  description = "NixOS environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShell.${system} = with pkgs;
      mkShell {
        ###
        ## Executable Packages
        ###

        buildInputs = with pkgs; [
          clang
          # Replace llvmPackages with llvmPackages_X, where X is the latest
          # LLVM version (at the time of writing, 16)
          llvmPackages_16.bintools
          mold
          pkg-config
          rustup
          yq-go
        ];

        ###
        ## Rust Toolchain Setup
        ###

        shellHook = ''
          export RUSTC_VERSION=$(yq ".toolchain.channel" rust-toolchain.toml)
          export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
          export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
          rustup component add rust-analyzer
        '';

        ###
        ## Rust Bindgen Setup
        ###

        # So bindgen can find libclang.so
        LIBCLANG_PATH = pkgs.lib.makeLibraryPath [pkgs.llvmPackages_16.libclang.lib];
        # Add headers to bindgen search path
        BINDGEN_EXTRA_CLANG_ARGS =
          # Includes with normal include path
          builtins.map (a: ''-I"${a}/include"'') [
            # add dev libraries here (e.g. pkgs.libvmi.dev)
            pkgs.glibc.dev
          ];
      };
  };
}
