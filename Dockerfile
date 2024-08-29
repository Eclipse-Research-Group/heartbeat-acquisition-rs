FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:edge@sha256:3e1def581eb9c9f11cfff85745802f2de5cf9cdeeb5a8495048f393a0993b99b
#FROM rust:bullseye as cross-base
ENV DEBIAN_FRONTEND=noninteractive
# COPY .cargo/config.toml /root/.cargo/config.toml

RUN dpkg --add-architecture armhf

RUN apt-get update

RUN apt-get install -y build-essential cmake libudev-dev:armhf libc6-dev-armhf-cross libssl-dev:armhf

#RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
#    gcc-arm-linux-gnueabihf \
#    g++-arm-linux-gnueabihf \
#    libc6-dev-armhf-cross \
#    libudev-dev:armhf \
#    libssl-dev:armhf \
#    build-essential \
#    make \ 
#    cmake

