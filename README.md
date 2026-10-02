# Flashpoint: Privileged-Software Isolation on RISC-V

Flashpoint is a hardware-software co-design for isolating RISC-V M-mode
software. It combines a lightweight ISA extension with **Anchor**, a small Rust
mediator that establishes a dynamic root of trust for the Tyche security
monitor, isolates Tyche from OpenSBI, and records measurements in a TPM.

This artifact supports three experiments:

1. Run the complete Flashpoint DRTM workflow in QEMU with `swtpm`.
2. Run Flashpoint on a modified XiangShan core on a Xilinx U55C FPGA.
3. Verify Flashpoint's isolation guarantees with Kani.

Source repository: [epfl-dcsl/flashpoint](https://github.com/epfl-dcsl/flashpoint).

## Artifact at a glance

| Item | Details |
| --- | --- |
| Host | x86-64 machine with Docker and at least 50 GB of free disk space |
| Container | Ubuntu 22.04; Docker tested with version 28.2.2 |
| Toolchains (Set up automatically during Docker image build) | Rust nightly-2025-07-14, Tyche Rust nightly-2023-12-01, RISC-V GNU GCC 11.4.0, and Just 1.23.0 |
| Docker image build time | About 30 minutes |
| QEMU experiment (in Docker container) | Less than 30 minutes |
| FPGA experiment (on provided hardware) | Less than 30 minutes |
| Kani verification (on provided hardware) | Less than 15 minutes of human time and about 15 hours of compute time |
| Evaluation format | Scripts to inspect console logs, cycle and instruction counts, and verification times |

The Docker image installs the required Rust and RISC-V toolchains, recursively
clones the repository and its submodules, builds `swtpm`, and downloads the
prebuilt Linux images.

## Build the Docker image

You do NOT need to clone the repository. 

Build the Docker image on your x86-64 host machine with Docker installed and at least 50 GB of free disk space using the command given below. 

Download the
[`Dockerfile`](Dockerfile) into an otherwise empty directory, open a terminal in
that directory, and run:

```sh
docker build --platform linux/amd64 --ulimit nofile=65536:65536 \
  -t flashpoint-workspace .
```

The build requires less than 5 minutes of human time and approximately 30
minutes of compute time.

Start a container:

```sh
docker run -it --rm --name flashpoint-dev \
  --platform linux/amd64 \
  --ulimit nofile=65536:65536 \
  --workdir /flashpoint \
  flashpoint-workspace /bin/bash
```

The container opens in the repository root. Confirm that the expected TPM
emulator is installed:

```sh
swtpm --version
```

The output should begin with:

```text
TPM emulator version 0.9.0
```

## Experiment 1: QEMU DRTM with `swtpm`

**Time required:** less than 30 minutes.

This experiment boots Anchor, the TPM driver, OpenSBI, Tyche, and Linux in a
modified QEMU. The TPM driver measures the initial PMP state, Anchor, the TPM
driver, and Tyche.

Run the following commands inside the container:

```sh
just build-qemu-software
just run-drtm-qemu 2>&1 | tee my_qemu_run_log.txt
```

Linux should boot with Tyche and open a shell. No command needs to be run in the
guest. Exit QEMU by pressing <kbd>Ctrl</kbd>+<kbd>A</kbd>, then <kbd>X</kbd>.
The complete console output remains in `my_qemu_run_log.txt`.

### Validate the PMP configuration

Extract and sort the PMP logs, which may arrive out of order from different
harts:

```sh
python3 scripts/extract_pmp_logs.py my_qemu_run_log.txt \
  --output extracted_pmp_logs.txt
```

Compare the extracted configuration with the expected result:

```sh
diff -u expected_results/pmp_logs_figure_4.txt extracted_pmp_logs.txt
```

Successful validation produces no diff. 

See
[`docs/PMP_Configuration.md`](docs/PMP_Configuration.md) for a detailed explanation of
how the log corresponds to Figure 4 in the paper.

### Verify TPM measurements

Check the measurements reported by the TPM against the built Anchor, TPM driver,
and Tyche ELF files:

```sh
python3 scripts/verify_srtm_drtm_measurements.py \
  --log my_qemu_run_log.txt
```

The script reports `[PASS]` for each matching measurement. A reference output is
available in
[`expected_results/verify-srtm-drtm-result.txt`](expected_results/verify-srtm-drtm-result.txt).

Keep the container running if you plan to build the FPGA software next.

## Experiment 2: XiangShan FPGA

**Time required:** less than 30 minutes.

This experiment uses a modified XiangShan core on a Xilinx U55C FPGA. The
artifact includes a prebuilt bitstream because synthesizing a new bitstream
takes substantial time.

FPGA access is provided to artifact evaluators. 

### Build the software image

Inside the Docker container, run:

```sh
just build-xiangshan-software
```

While the container is still running, open another terminal on the host and copy
out the generated binary:

```sh
docker cp \
  flashpoint-dev:/flashpoint/xiangshan/xiangshan_software.bin \
  .
```

The container uses `--rm`, so files left inside it are deleted when it exits.
After copying the binary, you may exit the container.

### Program and run the FPGA

For instructions to generate the bitstream, program the board, load
`xiangshan_software.bin`, and monitor its output, follow the
[FPGA setup and execution guide](fpga_scripts/README.md). The exact steps may
vary depending on how the FPGA, programming, and reset interfaces are connected
in your setup.

The console output should resemble
[`expected_results/flashpoint-xiangshan-run-log.txt`](expected_results/flashpoint-xiangshan-run-log.txt).

To leave Minicom, press <kbd>Ctrl</kbd>+<kbd>A</kbd>, then <kbd>Z</kbd>,
<kbd>X</kbd>, and <kbd>Enter</kbd>.

### Verify boot overhead

The boot logs from the Anchor begin with :

```text
[INFO | anchor] Boot time measurements for hart 0:
```

It should also contain the total M-mode execution:

```text
[INFO | tyche::riscv] HartID: 0 Mcycle : 676995571 Minstret 102677344
```

The former logs indicate the execution overhead of the Anchor, TPM driver, and firmware while the latter is the total M-mode execution before first switch to Linux.

Copy the boot log in a text file, and then this script computes the overhead in percentage.
```sh
./scripts/m-mode-boot-proportions.sh expected_results/flashpoint-xiangshan-run-log.txt 
```

Output looks like: 

```
Region                        % mcycle   % minstret
Anchor                       0.005219%    0.007454%
TPM driver (SRTM)            0.000391%    0.000167%
Untrusted firmware          46.323093%   89.801057%
TPM driver (DRTM)            0.000282%    0.000109%
Tyche                       53.670817%   10.191091%
```

The counts will vary across boots, but will not affect the negligible overhead of the Anchor during boot time (as reported in Table 3 in the paper). 

### Run the benchmarks

Run the following workloads from the FPGA's Linux shell after boot.

#### CoreMark (less than 5 minutes)

```sh
coremark
```

Reference output:
[`expected_results/coremark-benchmarks.txt`](expected_results/coremark-benchmarks.txt).

#### Tyche microbenchmarks (less than 5 minutes)

```sh
cd tychebench
sh riscv64-install-drivers.sh
cd benchmarks
CREATION=TRUE CARVES=TRUE OUTER=10 INNER=1 ./runbench
CREATION=TRUE SANDBOXES=TRUE OUTER=10 INNER=1 ./runbench
CREATION=TRUE ENCLAVES=TRUE OUTER=10 INNER=1 ./runbench
cd ../..
```

Reference output:
[`expected_results/tyche-microbenchmarks.txt`](expected_results/tyche-microbenchmarks.txt).

#### RV8 (less than 10 minutes)

```sh
cd rv8
bash bench2.sh bench all native-riscv64 O3 1
```

Reference output:
[`expected_results/rv8-benchmarks.txt`](expected_results/rv8-benchmarks.txt).

## Experiment 3: Kani verification

**Time required:** less than 15 minutes of human time and approximately 15 hours of compute time.

The complete model-checking run requires a machine with at least 64 GB of RAM, with Kani version 0.67.0.

On the provided server, run:

```sh
cd flashpoint
tmux new -s kani-verif-flashpoint
just verif-kani-anchor 2>&1 | tee kani-log.txt
```

Detach from tmux by pressing <kbd>Ctrl</kbd>+<kbd>B</kbd>, then <kbd>D</kbd>. The
run takes approximately 15 hours.

Check again after approximately 15 hours. The end of the log should report:

```text
Complete - 6 successfully verified harnesses, 0 failures, 6 total.
```

Confirm that each harness completed in less than three hours, as reported in
Section 7 of the paper:

```sh
python3 scripts/extract_verif_time.py kani-log.txt
```

A complete reference log is available in
[`expected_results/kani-run-log.txt`](expected_results/kani-run-log.txt).

## TROUBLESHOOTING 

### Dockerfile 

If recursive submodule checkout fails intermittently with GitHub authentication or rate-limit errors, retry using shallow clones as follows: 

```sh
git submodule update --init --recursive --depth 1
```

The following command can be used to check if any submodules are still uninitialized, if any entry still starts with '-'.

```sh
git submodule status --recursive
```

You can even use a retry loop as below, which only retries the missing submodules. REPO_REF = main. 

```sh
RUN test -n "${REPO_URL}" \
 && git clone "${REPO_URL}" flashpoint \
 && git -C flashpoint checkout "${REPO_REF}" \
 && cd /flashpoint \
 && for i in 1 2 3 4 5 6 7 8; do \
 git submodule sync --recursive; \
 git submodule update --init --recursive --depth 1 && break; \
 sleep 45; \
 done \
 && git submodule status --recursive \
 | awk '/^-/ {print "UNINITIALISED: " $0; bad=1} END {exit bad+0}'
```

### Kani 

Kani also requires storage for temporary computations. 
In case the storage in your filesystem root is full, you may specify a different storage directory <storage-path> using the following: 

```sh
mkdir -p <storage-path>/kani-tmp
mkdir -p <storage-path>/kani-target
chmod 700 <storage-path>/kani-tmp
TMPDIR=<storage-path>/kani-tmp 
CARGO_TARGET_DIR=<storage-path>/kani-target 
just verif-kani-anchor 2>&1 | tee kani-log.txt
```

## License

Flashpoint is distributed under the [MIT License](LICENSE). Bundled third-party
projects retain their respective licenses.
