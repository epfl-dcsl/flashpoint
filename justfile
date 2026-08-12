
build_std := "-Zbuild-std=core,alloc"
build_features := "-Zbuild-std-features=compiler-builtins-mem"
cargo_args := build_std + " " + build_features
riscv := "--target ./configs/riscv-unknown-kernel.json"

# ************************************ Software target bins ***************************** #

# debug target path 
debug_target_path := "target/riscv-unknown-kernel/debug"

# anchor
anchor_cargo_package := "-p anchor"
anchor_rustflags := "RUSTFLAGS='-C link-arg=-T./configs/anchor-linker-script.x'"
anchor_elf := debug_target_path + "/anchor"
anchor_img_qemu := debug_target_path + "/anchor_qemu.img"
anchor_img_xiangshan := debug_target_path + "/anchor_xiangshan.img"

# tpm-driver
tpm-driver_cargo_package := "-p tpm-driver"
tpm_drv_rustflags := "RUSTFLAGS='-C link-arg=-T./configs/tpm_driver-linker-script.x'"
tpm_drv_elf := debug_target_path + "/tpm-driver"
tpm_drv_img_qemu := debug_target_path + "/tpm-driver_qemu.img"
tpm_drv_img_xiangshan := debug_target_path + "/tpm-driver_xiangshan.img"

# ************************************ QEMU ********************************************* #

qemu-path := "./qemu/build/qemu-system-riscv64"
osbi-tyche-loader := "loader,file=./tyche/opensbi-stage1/build/platform/generic/firmware/fw_payload.bin,addr=0x80200000,force-raw=on"
osbi-tyche-elf := "./tyche/opensbi-stage1/build/platform/generic/firmware/fw_payload.elf"
tpm_path := "./tpm"
tpm-dev   := "-device tpm-tis-device,tpmdev=tpm0 -tpmdev emulator,id=tpm0,chardev=tpm-chardev -chardev socket,id=tpm-chardev,path=./tpm/sock"
tpm-driver-loader := "loader,file=./target/riscv-unknown-kernel/debug/tpm-driver_qemu.img,addr=0x80080000,force-raw=on"

# ************************************ Features ***************************************** #

perf_counters := "--features perf_counters"
xiangshan := "--features xiangshan"
softcore := "--features softcore"

# ************************************ Verification ************************************* #

kani_version := "0.67.0"

# Run the Anchor unit tests using softcore
verif-test-anchor:
    cargo test {{ anchor_cargo_package }} {{ softcore }} -- --test-threads=1

# Run the Kani model-checker on Anchor
verif-kani-anchor:
    cargo kani {{ anchor_cargo_package }} --features softcore,loglevel-off --output-format terse

# ************************************ Targets ****************************************** #

# Setup Rust toolchain, Tyche, swtpm, and apply Flashpoint patch to QEMU 
setup:
    @just _setup-rust-toolchain
    @just _setup-tyche
    @just _setup-swtpm
    @just _setup-qemu-with-patch

# Build QEMU, Anchor, TPM Driver, Tyche
build-qemu-software:
    cargo clean 
    @just clean-tyche
    @just build-qemu
    @just build-anchor-qemu
    @just build-tpm-driver-qemu
    @just build-tyche-qemu

# Build Anchor, TPM Driver, Tyche for XiangShan 
build-xiangshan-software:
    cargo clean 
    @just clean-tyche
    @just build-anchor-xiangshan
    @just build-tpm-driver-xiangshan
    @just build-tyche-xiangshan

# Build QEMU with Flashpoint ISA extension 
build-qemu:
	./qemu/build_qemu.sh

# Build Tyche for QEMU with OpenSBI
build-tyche-qemu:  
	just -f tyche/justfile build-riscv

# Build Tyche for XiangShan 
build-tyche-xiangshan:
    just -f tyche/justfile build-riscv-xiangshan
    ./scripts/opensbi_build_xiangshan.sh
    mv xiangshan_software.bin xiangshan/

# Build the Anchor for QEMU
build-anchor-qemu:
    {{ anchor_rustflags }} cargo build {{ anchor_cargo_package }} {{ riscv }} {{ cargo_args }} {{ perf_counters }}
    rust-objcopy -O binary {{ anchor_elf }} {{ anchor_img_qemu }}

# Build the Anchor for XiangShan
build-anchor-xiangshan:
    {{ anchor_rustflags }} FLASHPOINT_NUM_HARTS=1 cargo build {{ anchor_cargo_package }} {{ riscv }} {{ cargo_args }} {{ xiangshan }} {{ perf_counters }}
    rust-objcopy -O binary {{ anchor_elf }} {{ anchor_img_xiangshan }}

# Build the TPM driver for QEMU
build-tpm-driver-qemu:
    make -C tpm-driver/crates/tpm_driver/C_files
    mkdir -p $(pwd)/bindings
    export RUST_BACKTRACE=1
    BIND_DIR=$(pwd)/bindings TPM_DRIVER_SOURCE_DIR=$(pwd)/tpm-driver/crates/tpm_driver/C_files/ {{ tpm_drv_rustflags }} cargo build {{ tpm-driver_cargo_package }} {{ riscv }} {{ cargo_args }} {{ perf_counters }}
    rust-objcopy -O binary {{ tpm_drv_elf }} {{ tpm_drv_img_qemu }}

# Build the TPM driver shim layer for XiangShan
build-tpm-driver-xiangshan:
    make -C tpm-driver/crates/tpm_driver/C_files
    mkdir -p $(pwd)/bindings
    export RUST_BACKTRACE=1
    BIND_DIR=$(pwd)/bindings TPM_DRIVER_SOURCE_DIR=$(pwd)/tpm-driver/crates/tpm_driver/C_files/ {{ tpm_drv_rustflags }} cargo build {{ tpm-driver_cargo_package }} {{ riscv }} {{ cargo_args }} {{ xiangshan }}
    rust-objcopy -O binary {{ tpm_drv_elf }} {{ tpm_drv_img_xiangshan }}

# Run the DRTM Setup on QEMU with swtpm
run-drtm-qemu:
    @just _start-tpm
    {{ qemu-path }} --no-reboot --nographic -M virt -cpu rv64,xflashpoint=on -bios {{ anchor_img_qemu }} -device {{ tpm-driver-loader }} -device {{ osbi-tyche-loader }} -smp 2 {{ tpm-dev }}

# Clean tyche 
clean-tyche: 
    cd tyche && cargo clean
    # Todo: can also clean up the initramfs/linux here with: rm -rf tyche/builds/*

# Build Linux (in the Tyche tree) for QEMU 
build-linux-qemu:
    just -f tyche/justfile init-ramfs-riscv # requires sudo to create devices in the ramfs
    just -f tyche/justfile build-busybox-riscv
    just -f tyche/justfile build-linux-riscv

# Build Linux for XiangShan with Tyche drivers 
build-linux-xiangshan:
    #!/usr/bin/env bash
    set -euo pipefail # To stop if a command fails or a variable is undefined
    git -C tyche/linux checkout neelu_xiangshan_dev
    export RISCV_ROOTFS_HOME="{{justfile_directory()}}/xiangshan/riscv-rootfs"
    export RISCV_LINUX_HOME="{{justfile_directory()}}/tyche/linux"
    export ARCH=riscv
    export CROSS_COMPILE=riscv64-linux-gnu-
    make -C "$RISCV_LINUX_HOME" xiangshan_defconfig
    make -C "$RISCV_LINUX_HOME" -j"$(nproc)"
    cp "$RISCV_LINUX_HOME/arch/riscv/boot/Image" "{{justfile_directory()}}/xiangshan/Image"

# Apply Flashpoint ISA extension patch to sail-riscv 
_setup-sail-riscv-with-patch:
    #!/usr/bin/env sh
    patch="$(pwd)/patches/sail-riscv.patch"

    if git -C sail-riscv apply --check "$patch" 2>/dev/null; then
        git -C sail-riscv apply "$patch"
        echo "Patch applied"
    elif git -C sail-riscv apply --reverse --check "$patch" 2>/dev/null; then
        echo "Patch is already applied"
    else
        echo "Patch is neither cleanly applicable nor fully applied" >&2
        echo "Details:" >&2
        git -C sail-riscv apply --check "$patch"
        exit 1
    fi

# Start running the TPM emulator (swtpm)
_start-tpm:
    #!/usr/bin/env sh
    if pgrep -x swtpm;
    then
        echo "TPM is running"
    else
        echo "Starting TPM with RISC-V arguments"
        mkdir -p {{tpm_path}}/
        # Add --seccomp action=none when running in the docker environment 
        swtpm socket --tpm2 --tpmstate dir={{tpm_path}} --ctrl type=unixio,path={{tpm_path}}/sock --seccomp action=none --log file={{tpm_path}}/logs,level=5 --locality allow-set-locality &
        # swtpm socket --tpm2 --tpmstate dir={{tpm_path}} --ctrl type=unixio,path={{tpm_path}}/sock --log file={{tpm_path}}/logs,level=5 --locality allow-set-locality &
    fi
    sleep 1

# Setup rust-toolchain for Tyche 
_setup-tyche: 
    rustup toolchain install nightly-2023-12-01     --profile minimal     --component rust-src    

# Build swtpm v0.9.0 with libtpms v0.10.0
_setup-swtpm:
    ./scripts/build_swtpm.sh

# Apply Flashpoint ISA extension patch to QEMU 
_setup-qemu-with-patch:
    #!/usr/bin/env sh
    patch="$(pwd)/patches/qemu-flashpoint.patch"

    if git -C qemu apply --check "$patch" 2>/dev/null; then
        git -C qemu apply "$patch"
        echo "Patch applied"
    elif git -C qemu apply --reverse --check "$patch" 2>/dev/null; then
        echo "Patch is already applied"
    else
        echo "Patch is neither cleanly applicable nor fully applied" >&2
        echo "Details:" >&2
        git -C qemu apply --check "$patch"
        exit 1
    fi

# Install the rust toolchain and required components
_setup-rust-toolchain:
    rustup toolchain install $(cat rust-toolchain)
    rustup target add riscv64gc-unknown-linux-gnu
    rustup component add rustfmt --toolchain "$(cat rust-toolchain)"
    rustup component add rust-src --toolchain "$(cat rust-toolchain)"
    rustup component add llvm-tools-preview --toolchain "$(cat rust-toolchain)"
    rustup component add clippy --toolchain "$(cat rust-toolchain)"
    cargo install cargo-binutils
    cargo install --locked just
    cargo install --locked kani-verifier --version {{ kani_version }}
    cargo kani setup

