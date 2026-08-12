# Flashpoint: Privileged-Software Isolation on RISC-V with a Hardware-Software Co-design

## Setup using the Dockerfile 

Build the docker image using the provided Dockerfile, you don't need to clone the repo, the Dockerfile will do that when building the image. 
It will also install all dependencies and recursively update the submodules, run the basic setup command (```just setup```) which sets up the rust toolchain, builds the required swtpm (TPM emulator) version, and finally downloads the prebuilt linux images. 

```docker build --platform linux/amd64 --ulimit nofile=65536:65536 --progress=plain -t flashpoint-workspace .```

Once the image is built, run the following command to run a new container from the flashpoint-workspace image: 

```docker run -it --rm --name flashpoint-dev --platform linux/amd64 --ulimit nofile=65536:65536 --workdir /flashpoint flashpoint-workspace /bin/bash```

A simple test to check that the image is built as expected: (run from within the container) 

```swtpm --version```

Output: ```TPM emulator version 0.9.0, Copyright (c) 2014-2022 IBM Corp. and others```

## Setup without using the Dockerfile 


## Run Flashpoint on QEMU with swtpm (TPM emulator)

```just build-qemu-software```

```just run-drtm-qemu```

Note: swtpm --seccomp=none vs. not (to appropriately change in justfile)

## Run Flashpoint on the XiangShan Core on FPGA 

```just build-xiangshan-software```

To copy out the xiangshan software binary, go to the directory where you want to copy it in the host, and then: (run this command outside the container, while the container is active, since "--rm" will delete the container and not preserve any changes to the image)

```docker cp flashpoint-dev:/flashpoint/xiangshan/xiangshan_software.bin .```