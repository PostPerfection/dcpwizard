FROM registry.fedoraproject.org/fedora:42

ARG NODE_VERSION=24.21.0
ARG NODE_SHA256=fd8e59d5a511510f6a298afb548f18c7d2b1be404d8b4a27d94fbe49f56cb2d6
ARG PNPM_VERSION=11.8.0
ARG FFMPEG_MPV_RELEASE=v1.0.0
ARG FFMPEG_MPV_ARCHIVE=ffmpeg-mpv-linux-x86_64.tar.xz
# NVIDIA's fedora42 repo stops at CUDA 13.1
ARG CUDA_REPOSITORY=https://developer.download.nvidia.com/compute/cuda/repos/fedora43/x86_64/cuda-fedora43.repo
ARG CUDA_PACKAGE_SUFFIX=13-2
ARG CUDA_DIRECTORY=/usr/local/cuda-13.2

RUN dnf install -y \
        gcc-c++ \
        make \
        cmake \
        ninja-build \
        clang-devel \
        pkgconf-pkg-config \
        libxml2-devel \
        openssl-devel \
        xerces-c-devel \
        alsa-lib-devel \
        webkit2gtk4.1-devel \
        libappindicator-gtk3-devel \
        librsvg2-devel \
        patchelf \
        libtiff-devel \
        libcurl-devel \
        binutils \
        git \
        curl \
        jq \
        rsync \
        xz \
        tar \
    && dnf clean all

RUN dnf config-manager addrepo --from-repofile="$CUDA_REPOSITORY" \
    && dnf install -y \
        "cuda-nvcc-$CUDA_PACKAGE_SUFFIX" \
        "cuda-cudart-devel-$CUDA_PACKAGE_SUFFIX" \
        "cuda-cuobjdump-$CUDA_PACKAGE_SUFFIX" \
    && dnf clean all

ENV PATH=$CUDA_DIRECTORY/bin:$PATH \
    CUDAToolkit_ROOT=$CUDA_DIRECTORY

ENV FFMPEG_MPV_DIR=/opt/ffmpeg-mpv

# the archive's libraries link against system ones that have to be there at link time
RUN base="https://github.com/PostPerfection/ffmpeg-mpv-builds/releases/download/$FFMPEG_MPV_RELEASE" \
    && mkdir -p /tmp/ffmpeg-mpv-download "$FFMPEG_MPV_DIR" \
    && cd /tmp/ffmpeg-mpv-download \
    && curl -fsSL --retry 5 --retry-all-errors -o SHA256SUMS "$base/SHA256SUMS" \
    && curl -fsSL --retry 5 --retry-all-errors -o "$FFMPEG_MPV_ARCHIVE" "$base/$FFMPEG_MPV_ARCHIVE" \
    && grep "  $FFMPEG_MPV_ARCHIVE\$" SHA256SUMS > expected.sha256 \
    && sha256sum -c expected.sha256 \
    && tar -C "$FFMPEG_MPV_DIR" --strip-components=1 -xJf "$FFMPEG_MPV_ARCHIVE" \
    && rm -rf /tmp/ffmpeg-mpv-download \
    && find "$FFMPEG_MPV_DIR/lib" -maxdepth 1 -type f -name '*.so.*' -exec readelf -d {} + \
        | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' | sort -u \
        | while read -r soname; do [ -e "$FFMPEG_MPV_DIR/lib/$soname" ] || echo "$soname()(64bit)"; done \
        | xargs dnf install -y \
    && dnf clean all

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# the build runs as the host user
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
    && chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME" \
    && rustc --version \
    && cargo --version

RUN curl -fsSLo /tmp/node.tar.xz "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
    && echo "${NODE_SHA256}  /tmp/node.tar.xz" | sha256sum -c - \
    && tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1 --no-same-owner \
    && rm /tmp/node.tar.xz \
    && npm install -g "pnpm@${PNPM_VERSION}" \
    && node --version \
    && pnpm --version
