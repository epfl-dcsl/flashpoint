# Guide to Generate the FPGA Bitstream for the Xilinx U55C FPGA Board and Run Flashpoint on the FPGA

## Software dependencies 

We assume that you have installed all software dependencies required by the XiangShan project and Vivado. 

This flow has been tested only with Vivado v2024.2.


## Setup 

This directory contains all scripts required to generate the bitstream:

Clone the required XiangShan source code and apply our patches (present in `flashpoint-patches/`) that implement the Flashpoint instructions:

```shell
sh download_xiangshan.sh
```

## Generating the Bitstream

Generate the Verilog files for the Flashpoint-enabled version of XiangShan, create a Vivado project, and generate the bitstream for the U55C board:

```shell
sh gen_bit.sh
vivado -mode batch -source build_bit.tcl
```

> This step can take approximately 2 hours and requires about 32 GB of RAM.

After the build completes, the bitstream and configuration files can be found under `project_X/project_X.runs/impl_1`:
- `u55c_top.bit`
- `u55c_top.ltx`

## Using the Bitstream

We assume that you have two dedicated Linux machines for using this bitstream:
- **Machine A:** Connected to the PCIe interface of the U55C board.
- **Machine B:** Connected to the JTAG/UART interface of the U55C board.

On Machine A, the bitstream exposes an XDMA device for writing the test software to the FPGA memory.

On Machine B, the bitstream exposes a VIO interface for resetting and releasing the XiangShan core on the FPGA, as well as a UART interface for monitoring the output of software running on the XiangShan core.

## Run Software on the FPGA Board

> The following steps and scripts are provided for reference only. We do not guarantee that they will work without modification on arbitrary machines.

We assume that Machine B is also the machine used to generate the bitstream.

Program the FPGA from Machine B using the following script in the `anchor-fpga` repository:

```shell
vivado -mode batch -source program_u55c.tcl
```

Still on Machine B, reset the XiangShan core by executing the following Tcl script from the `anchor-fpga` repository:

```shell
vivado -mode batch -source set_u55c_vio_zero.tcl
```

On Machine A, compile and load the [XDMA driver](https://github.com/Xilinx/dma_ip_drivers) according to the instructions in the driver repository.

Still on Machine A, rescan the PCIe bus to detect the XDMA device:

```shell
# The first command may fail, but this does not affect the subsequent rescan.
echo 1 > /sys/class/pci_bus/0000\:01/device/remove
echo 1 > /sys/bus/pci/rescan
```

Next, clone the [XDMA helper program](https://github.com/wengwz/Xilinx-XDMA-Host-Software.git) and compile the host utility on Machine A:

```shell
git clone https://github.com/wengwz/Xilinx-XDMA-Host-Software.git
cd Xilinx-XDMA-Host-Software
make host
```

After compilation, you will find the `xdma_rw` binary under `Xilinx-XDMA-Host-Software/bin`. We use this utility to load test programs onto the FPGA board.

Load a binary program onto the FPGA with:

```shell
sudo ./xdma_rw -w -d /dev/xdma0_h2c_0 -f [SOFTWARE] -s [SIZE] -c 1 -a 0x80000000
```

Here, `SOFTWARE` is the path to the binary program, and `SIZE` is the size of the binary in bytes.

On Machine B, open a Minicom console to monitor the program output:

```shell
minicom -D /dev/ttyUSB2
```

> The UART device name (`ttyUSB2`) may differ on your system. If this device does not work, check the available USB UART devices on Machine B.

Still on Machine B, open another terminal and release the XiangShan core by executing the following script from the `anchor-fpga` repository:

```shell
vivado -mode batch -source set_u55c_vio_one.tcl
```

The software output should then appear in the Minicom console.

