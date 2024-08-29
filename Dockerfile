# FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:edge@sha256:3e1def581eb9c9f11cfff85745802f2de5cf9cdeeb5a8495048f393a0993b99b
FROM rust:bullseye as cross-base
ENV DEBIAN_FRONTEND=noninteractive
# COPY .cargo/config.toml /root/.cargo/config.toml

RUN dpkg --add-architecture armhf

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    gcc-arm-linux-gnueabihf \
    g++-arm-linux-gnueabihf \
    libc6-dev-armhf-cross \
    libudev-dev:armhf \
    libssl-dev:armhf \
    build-essential \
    make \ 
    cmake

ENV CROSS_TOOLCHAIN_PREFIX=arm-linux-gnueabihf-
ENV CROSS_SYSROOT=/usr/arm-linux-gnueabihf
ENV CROSS_TARGET_RUNNER="/linux-runner armv7hf"
ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="$CROSS_TOOLCHAIN_PREFIX"gcc \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_RUNNER="$CROSS_TARGET_RUNNER" \
    AR_armv7_unknown_linux_gnueabihf="$CROSS_TOOLCHAIN_PREFIX"ar \
    CC_armv7_unknown_linux_gnueabihf="$CROSS_TOOLCHAIN_PREFIX"gcc \
    CXX_armv7_unknown_linux_gnueabihf="$CROSS_TOOLCHAIN_PREFIX"g++ \
    CMAKE_TOOLCHAIN_FILE_armv7_unknown_linux_gnueabihf=/opt/toolchain.cmake \
    BINDGEN_EXTRA_CLANG_ARGS_armv7_unknown_linux_gnueabihf="--sysroot=$CROSS_SYSROOT" \
    QEMU_LD_PREFIX="$CROSS_SYSROOT" \
    RUST_TEST_THREADS=1 \
    PKG_CONFIG_PATH="/usr/lib/arm-linux-gnueabihf/pkgconfig/:${PKG_CONFIG_PATH}" \
    PKG_CONFIG_ALLOW_CROSS=1 \
    PKG_CONFIG_SYSROOT_DIR="/usr/lib/arm-linux-gnueabihf/" \
    CROSS_CMAKE_SYSTEM_NAME=Linux \
    CROSS_CMAKE_SYSTEM_PROCESSOR=arm \
    CROSS_CMAKE_CRT=gnu \
    CMAKE_C_COMPILER="$CROSS_TOOLCHAIN_PREFIX"gcc \
    CMAKE_CXX_COMPILER="$CROSS_TOOLCHAIN_PREFIX"g++ \
    CROSS_CMAKE_OBJECT_FLAGS="-ffunction-sections -fdata-sections -fPIC -march=armv7-a -mfpu=vfpv3-d16"

COPY toolchain.cmake /opt/toolchain.cmake

WORKDIR /project

RUN rustup target add armv7-unknown-linux-gnueabihf

ENTRYPOINT ["cargo"]