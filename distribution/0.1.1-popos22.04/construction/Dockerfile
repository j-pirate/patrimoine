# syntax=docker/dockerfile:1
FROM ubuntu:22.04@sha256:74b12b11ffa550fba1929ad4d8d16533c34f6612728469671afba1b03b66881c
RUN --mount=type=secret,id=system_ca,required=true,mode=0444 \
    sed -i 's|http://archive.ubuntu.com|https://archive.ubuntu.com|g; s|http://security.ubuntu.com|https://archive.ubuntu.com|g' /etc/apt/sources.list \
    && apt-get -o Acquire::https::CaInfo=/run/secrets/system_ca update \
    && DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::https::CaInfo=/run/secrets/system_ca install -y --no-install-recommends \
        build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev \
        patchelf curl ca-certificates file xdg-utils squashfs-tools \
        webkit2gtk-driver xvfb xauth dbus-x11 python3 xdotool fonts-dejavu-core \
        git unzip \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --create-home patrimoine
ENV LANG=C.UTF-8
USER 1000:1000
WORKDIR /work
