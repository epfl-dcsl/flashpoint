FROM ubuntu:22.04

ENV TZ=Europe/Zurich
ENV DEBIAN_FRONTEND=noninteractive

# General build, RISC-V, Linux, and QEMU dependencies
RUN ln -snf "/usr/share/zoneinfo/${TZ}" /etc/localtime \
    && echo "${TZ}" > /etc/timezone \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    build-essential \
    sudo \
    iproute2 \
    rsync \
    curl \
    git \
    wget \
    clang \
    libclang-dev \
    llvm-dev \
    vim \
    diffutils \
    python3 \
    python3-venv \
    python3-tomli \
    python3-setuptools \
    device-tree-compiler \
    gcc-riscv64-linux-gnu \
    libc6-dev-riscv64-cross \
    gdb-multiarch \
    bc \
    autoconf \
    automake \
    libtool \
    libtool-bin \
    bison \
    cmake \
    file \
    patch \
    unzip \
    xz-utils \
    ca-certificates \
    jq \
    libncurses-dev \
    flex \
    cpio \
    bear \
    libelf-dev \
    libssl-dev \
    musl-tools \
    ninja-build \
    pkg-config \
    libglib2.0-dev \
    libpixman-1-dev \
    zlib1g-dev \
    libtasn1-6-dev \
    libjson-glib-dev \
    expect \
    gawk \
    socat \
    libseccomp-dev \
    libgmp-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain nightly-2025-07-14

ENV PATH="/root/.cargo/bin:${PATH}"

# Install the Rust components and targets required
RUN cargo install just --locked --version 1.23.0

# For cloning without SSH keys
RUN git config --global url."https://github.com/".insteadOf "git@github.com:" \
    && git config --global url."https://git.busybox.net/".insteadOf "git://busybox.net/"

# REPO_REF may be a branch, tag, or an exact commit.
ARG REPO_URL="https://github.com/epfl-dcsl/flashpoint.git"
ARG REPO_REF="main"

# Clone the parent repository and check out the exact submodule commits
WORKDIR /
RUN test -n "${REPO_URL}" \
    && git clone "${REPO_URL}" flashpoint \
    && git -C flashpoint checkout "${REPO_REF}" \
    && git -C flashpoint submodule update --init --recursive

# Run Flashpoint's setup recipe.
# Use SETUP_PATH=. if `just setup` should run in the parent repository.
# ENV USER=root
RUN cd "/flashpoint" && just setup

# TODO: Download and decompress linux images

RUN cd "/flashpoint" \
    && curl -fL "https://github.com/epfl-dcsl/flashpoint/releases/download/linux-v1/flashpoint-linux-qemu.tar.gz" -o flashpoint-linux-qemu.tar.gz \
    && tar -xzf flashpoint-linux-qemu.tar.gz -C . \
    && curl -fL   "https://github.com/epfl-dcsl/flashpoint/releases/download/linux-v1/flashpoint-linux-xiangshan.tar.gz"   -o flashpoint-linux-xiangshan.tar.gz \
    && tar -xzf flashpoint-linux-xiangshan.tar.gz -C .

WORKDIR /flashpoint/
CMD ["/bin/bash"]
