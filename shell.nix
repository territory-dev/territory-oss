{
    rust-overlay ? builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/9e4b97a04063ff39a37b63e8fb31cc2b4adf9227.tar.gz",
    pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/041c867bad68dfe34b78b2813028a2e2ea70a23c.tar.gz") {
        overlays = [(import rust-overlay)];
    },
    rustToolchain ? pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml,
}:

let llvmPkgs = pkgs.llvmPackages_18;
in
llvmPkgs.stdenv.mkDerivation {
    name = "territory";

    buildInputs = [
      rustToolchain
      pkgs.pkg-config
      llvmPkgs.clang
      pkgs.libiconv
      llvmPkgs.bintools
      pkgs.protobuf
      pkgs.wasm-pack
      pkgs.curl
      pkgs.yarn
      pkgs.openssl.dev
      pkgs.python311
      pkgs.maturin
      pkgs.git
      pkgs.sqlite
      pkgs.cmake
      pkgs.expat.dev
      pkgs.tmux
      pkgs.go
      pkgs.gopls
      pkgs.protoc-gen-go
      pkgs.pcre2
      pkgs.cacert
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
      pkgs.darwin.apple_sdk.frameworks.Security
      pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
    ];

    #
    # see <https://nixos.wiki/wiki/Rust>
    #
    LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ llvmPkgs.libclang.lib ];
    CLANG_RESOURCE_DIR = ''${pkgs.lib.makeLibraryPath [ llvmPkgs.libclang.lib ]}/clang/18/include/'';

    # Add precompiled library to rustc search path
    RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
      # add libraries here (e.g. pkgs.libvmi)
    ]);
    # Add glibc, clang, glib and other headers to bindgen search path
    BINDGEN_EXTRA_CLANG_ARGS =
    # Includes with normal include path
    (builtins.map (a: ''-I"${a}/include"'') [
      # add dev libraries here (e.g. pkgs.libvmi.dev)
    ])
    # Includes with special directory paths
    ++ [
      ''-I"${llvmPkgs.libclang.lib}/lib/clang/${llvmPkgs.libclang.version}/include"''
      ''-I"${pkgs.glib.dev}/include/glib-2.0"''
      ''-I${pkgs.glib.out}/lib/glib-2.0/include/''
    ];

    CPLUS_INCLUDE_PATH = ''${llvmPkgs.libraries.libcxx.dev}/include/c++/v1/'';

    shellHook = ''
      export TT_REPO_ROOT="${ toString ./. }"

      if [ ! -d $TT_REPO_ROOT/.venv ]; then
        python -m venv $TT_REPO_ROOT/.venv
        $TT_REPO_ROOT/.venv/bin/pip install -r $TT_REPO_ROOT/requirements.txt
      fi

      source $TT_REPO_ROOT/.venv/bin/activate

      export PATH="$TT_REPO_ROOT/ops/dev:$TT_REPO_ROOT/target/release:$TT_REPO_ROOT/indexers/go:$PATH"

      if [[ $- == *i* ]]
      then
        echo "🚀 [⊤] 🚀"
        echo "  build_all.sh  Build all components"
      fi
    '';
}
