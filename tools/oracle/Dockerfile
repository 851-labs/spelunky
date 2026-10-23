# Build and run environment for the reference oracle (OpenGMK's gm8emulator on Linux, headless under Xvfb).
# The Rust image is pinned by version and digest; tools/oracle/build tags the result `spelunky-oracle`.
FROM rust:1.99.1-bookworm@sha256:48b7f435b8fc6378ce43b44f15e3a47fd8919b2825f835c39bd4bd3da50af5b8

# Build deps (audio and X11/xcb/GL headers for OpenGMK and ramen), then the headless runtime: Xvfb, Mesa's
# software GL (llvmpipe), and xdpyinfo to wait for the X server.
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    cmake pkg-config libasound2-dev libpulse-dev libjack-jackd2-dev \
    libx11-dev libx11-xcb-dev libxcb1-dev libxcb-xinput-dev libxkbcommon-dev libxkbcommon-x11-0 \
    libgl-dev libgl1-mesa-dri libglx-mesa0 xvfb x11-utils \
    && rm -rf /var/lib/apt/lists/*

# OpenGMK's rust-toolchain.toml asks for 1.77 (for Windows 7 support); build with the image's toolchain instead
# so nothing is downloaded at build time.
ENV RUSTUP_TOOLCHAIN=1.99.1
# Registry and git dependency caches live in a named volume mounted here (see common.sh).
ENV CARGO_HOME=/cargo
ENV PATH=/usr/local/cargo/bin:$PATH
ENV CARGO_TARGET_DIR=/target
# Upstream's own warnings (dozens) would bury real errors in the build output.
ENV RUSTFLAGS=-Awarnings
