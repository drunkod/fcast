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
                  if [ -d "/mnt/sda2/Android/Sdk" ]; then
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

                if [ -n "''${ANDROID_SDK_ROOT:-}" ] && [ -z "''${ANDROID_NDK_ROOT:-}" ] && [ -d "$ANDROID_SDK_ROOT/ndk" ]; then
                  newest_ndk="$(ls -1 "$ANDROID_SDK_ROOT/ndk" 2>/dev/null | sort -V | tail -n1)"
                  if [ -n "$newest_ndk" ] && [ -d "$ANDROID_SDK_ROOT/ndk/$newest_ndk" ]; then
                    export ANDROID_NDK_ROOT="$ANDROID_SDK_ROOT/ndk/$newest_ndk"
                    export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
                  fi
                fi

                if [ -n "''${ANDROID_NDK_ROOT:-}" ]; then
                  ndk_bin="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin"
                  if [ -x "$ndk_bin/aarch64-linux-android21-clang" ]; then
                    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ndk_bin/aarch64-linux-android21-clang"
                    export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$ndk_bin/llvm-ar"
                  fi
                fi

                if [ -z "''${ANDROID_SDK_ROOT:-}" ]; then
                  echo "warning: Android SDK not found. Set ANDROID_SDK_ROOT (recommended: /mnt/sda2/Android/Sdk)."
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
