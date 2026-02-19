{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs { inherit system; config = {}; overlays = []; };
          gst = pkgs.gst_all_1;
        in {
          packages = {
            fcast-sender = pkgs.callPackage ./senders/desktop/fcast-sender.nix { };
            default = self.packages.${system}.fcast-sender;
          };

          devShells = {
            default = pkgs.mkShell {
              buildInputs = with pkgs; [
                rustup
                cargo-ndk
                llvmPackages_18.clang
                llvmPackages_18.clang-unwrapped
                llvmPackages_18.llvm
                llvmPackages_18.lld
                pkg-config
                openssl
                android-tools
                wget
                unzip
                gnutar
                gnumake
                jdk17_headless
                gst.gstreamer
                gst.gst-plugins-base
                gst.gst-plugins-good
                gst.gst-plugins-bad
                gst.gst-libav
                glib
                pango
                cairo
                libxkbcommon
                wayland
                libx11
                libxext
                libxcursor
                libxi
                libxrandr
                libxcb
                vulkan-loader
              ];

              shellHook = ''
                export FCACHE_ROOT="''${FCACHE_ROOT:-/mnt/sda2/.fcast-dev}"
                export CARGO_HOME="''${CARGO_HOME:-$FCACHE_ROOT/cargo}"
                export RUSTUP_HOME="''${RUSTUP_HOME:-$FCACHE_ROOT/rustup}"
                mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
                export PATH="${pkgs.rustup}/bin:$CARGO_HOME/bin:$PATH"
                export RUSTUP_TOOLCHAIN=stable

                if ! rustup toolchain list 2>/dev/null | grep -q "^stable"; then
                  rustup toolchain install stable --profile minimal || true
                fi

                rustup component add rustfmt clippy --toolchain stable || true
                rustup target add \
                  aarch64-linux-android \
                  armv7-linux-androideabi \
                  x86_64-linux-android \
                  i686-linux-android \
                  --toolchain stable || true

                if [ -z "''${ANDROID_SDK_ROOT:-}" ]; then
                  if [ -d "$PWD/thirdparty/Android/Sdk" ]; then
                    export ANDROID_SDK_ROOT="$PWD/thirdparty/Android/Sdk"
                  elif [ -d "/mnt/sda2/projects/fcast/thirdparty/Android/Sdk" ]; then
                    export ANDROID_SDK_ROOT="/mnt/sda2/projects/fcast/thirdparty/Android/Sdk"
                  elif [ -d "/mnt/sda2/Android/Sdk" ]; then
                    export ANDROID_SDK_ROOT="/mnt/sda2/Android/Sdk"
                  elif [ -d "$HOME/Android/Sdk" ]; then
                    export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
                  fi
                fi

                if [ -n "''${ANDROID_SDK_ROOT:-}" ]; then
                  export ANDROID_HOME="$ANDROID_SDK_ROOT"
                fi

                if [ -n "''${ANDROID_SDK_ROOT:-}" ] && [ -z "''${ANDROID_JAR:-}" ]; then
                  newest_jar="$(ls -1 "$ANDROID_SDK_ROOT"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -n1)"
                  if [ -n "$newest_jar" ]; then
                    export ANDROID_JAR="$newest_jar"
                  fi
                fi

                if [ -z "''${ANDROID_NDK_ROOT:-}" ]; then
                  if [ -n "''${ANDROID_SDK_ROOT:-}" ] && [ -d "$ANDROID_SDK_ROOT/ndk" ]; then
                    newest_ndk="$(ls -1 "$ANDROID_SDK_ROOT/ndk" 2>/dev/null | sort -V | tail -n1)"
                    if [ -n "$newest_ndk" ] && [ -d "$ANDROID_SDK_ROOT/ndk/$newest_ndk" ]; then
                      export ANDROID_NDK_ROOT="$ANDROID_SDK_ROOT/ndk/$newest_ndk"
                    fi
                  elif [ -d "$PWD/thirdparty/android-ndk-r25c" ]; then
                    export ANDROID_NDK_ROOT="$PWD/thirdparty/android-ndk-r25c"
                  elif [ -d "/mnt/sda2/projects/fcast/thirdparty/android-ndk-r25c" ]; then
                    export ANDROID_NDK_ROOT="/mnt/sda2/projects/fcast/thirdparty/android-ndk-r25c"
                  fi
                fi

                if [ -n "''${ANDROID_NDK_ROOT:-}" ]; then
                  export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
                  ndk_prebuilt="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64"
                  ndk_sysroot="$ndk_prebuilt/sysroot"
                  ndk_clang_ver="$(ls -1 "$ndk_prebuilt/lib64/clang" 2>/dev/null | sort -V | tail -n1)"
                  ndk_clang_lib_root="$ndk_prebuilt/lib64/clang/$ndk_clang_ver/lib/linux"
                  host_clang="${pkgs.llvmPackages_18.clang-unwrapped}/bin/clang"
                  clang_resource_root="${pkgs.llvmPackages_18.clang}/resource-root/include"
                  host_llvm_ar="${pkgs.llvmPackages_18.llvm}/bin/llvm-ar"
                  toolchain_bin="$FCACHE_ROOT/android-toolchain/bin"
                  mkdir -p "$toolchain_bin"

                  write_cc_wrapper() {
                    wrapper="$1"
                    target="$2"
                    target_include="$3"
                    target_lib_dir="$4"
                    cat > "$wrapper" <<EOF
#!/usr/bin/env bash
exec "$host_clang" --target=$target --sysroot="$ndk_sysroot" \
  -nostdinc \
  -isystem "$clang_resource_root" \
  -isystem "$ndk_sysroot/usr/local/include" \
  -isystem "$ndk_sysroot/usr/include/$target_include" \
  -isystem "$ndk_sysroot/usr/include" \
  -L "$ndk_clang_lib_root/$target_lib_dir" \
  "\$@"
EOF
                    chmod +x "$wrapper"
                  }

                  write_cxx_wrapper() {
                    wrapper="$1"
                    target="$2"
                    target_include="$3"
                    target_lib_dir="$4"
                    cat > "$wrapper" <<EOF
#!/usr/bin/env bash
exec "$host_clang" --driver-mode=g++ --target=$target --sysroot="$ndk_sysroot" \
  -nostdinc \
  -isystem "$clang_resource_root" \
  -isystem "$ndk_sysroot/usr/include/c++/v1" \
  -isystem "$ndk_sysroot/usr/local/include" \
  -isystem "$ndk_sysroot/usr/include/$target_include" \
  -isystem "$ndk_sysroot/usr/include" \
  -L "$ndk_clang_lib_root/$target_lib_dir" \
  "\$@"
EOF
                    chmod +x "$wrapper"
                  }

                  if [ -x "$host_clang" ] && [ -x "$host_llvm_ar" ] && [ -d "$ndk_sysroot" ] && [ -d "$clang_resource_root" ] && [ -d "$ndk_clang_lib_root" ]; then
                    write_cc_wrapper "$toolchain_bin/aarch64-linux-android21-clang" "aarch64-linux-android21" "aarch64-linux-android" "aarch64"
                    write_cxx_wrapper "$toolchain_bin/aarch64-linux-android21-clang++" "aarch64-linux-android21" "aarch64-linux-android" "aarch64"
                    write_cc_wrapper "$toolchain_bin/armv7a-linux-androideabi21-clang" "armv7a-linux-androideabi21" "arm-linux-androideabi" "arm"
                    write_cxx_wrapper "$toolchain_bin/armv7a-linux-androideabi21-clang++" "armv7a-linux-androideabi21" "arm-linux-androideabi" "arm"
                    write_cc_wrapper "$toolchain_bin/x86_64-linux-android21-clang" "x86_64-linux-android21" "x86_64-linux-android" "x86_64"
                    write_cxx_wrapper "$toolchain_bin/x86_64-linux-android21-clang++" "x86_64-linux-android21" "x86_64-linux-android" "x86_64"
                    write_cc_wrapper "$toolchain_bin/i686-linux-android21-clang" "i686-linux-android21" "i686-linux-android" "i386"
                    write_cxx_wrapper "$toolchain_bin/i686-linux-android21-clang++" "i686-linux-android21" "i686-linux-android" "i386"

                    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$toolchain_bin/aarch64-linux-android21-clang"
                    export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$host_llvm_ar"
                    export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$toolchain_bin/armv7a-linux-androideabi21-clang"
                    export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR="$host_llvm_ar"
                    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$toolchain_bin/x86_64-linux-android21-clang"
                    export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="$host_llvm_ar"
                    export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$toolchain_bin/i686-linux-android21-clang"
                    export CARGO_TARGET_I686_LINUX_ANDROID_AR="$host_llvm_ar"

                    export CC_aarch64_linux_android="$toolchain_bin/aarch64-linux-android21-clang"
                    export CXX_aarch64_linux_android="$toolchain_bin/aarch64-linux-android21-clang++"
                    export CC_armv7_linux_androideabi="$toolchain_bin/armv7a-linux-androideabi21-clang"
                    export CXX_armv7_linux_androideabi="$toolchain_bin/armv7a-linux-androideabi21-clang++"
                    export CC_x86_64_linux_android="$toolchain_bin/x86_64-linux-android21-clang"
                    export CXX_x86_64_linux_android="$toolchain_bin/x86_64-linux-android21-clang++"
                    export CC_i686_linux_android="$toolchain_bin/i686-linux-android21-clang"
                    export CXX_i686_linux_android="$toolchain_bin/i686-linux-android21-clang++"
                  fi
                fi

                if [ -z "''${ANDROID_SDK_ROOT:-}" ]; then
                  echo "warning: Android SDK not found. Set ANDROID_SDK_ROOT (recommended: /mnt/sda2/projects/fcast/thirdparty/Android/Sdk)."
                fi

                export PKG_CONFIG_ALLOW_CROSS=1
                export PKG_CONFIG_PATH="${
                  pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
                    gst.gstreamer
                    gst.gst-plugins-base
                    gst.gst-plugins-good
                    gst.gst-plugins-bad
                    pkgs.openssl
                    pkgs.glib
                    pkgs.pango
                    pkgs.cairo
                    pkgs.libxkbcommon
                    pkgs.wayland
                    pkgs.libx11
                    pkgs.libxext
                    pkgs.libxcursor
                    pkgs.libxi
                    pkgs.libxrandr
                    pkgs.libxcb
                    pkgs.vulkan-loader
                  ]
                }:$PKG_CONFIG_PATH"
                export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
                export RUST_BACKTRACE=1
              '';
            };
          };
        }
      );
}
