cd tyche/opensbi-stage1/      
rm -rf build/*
make clean 
make PLATFORM=generic FW_FDT_PATH=../../configs/xiangshan.dtb TYCHE_SM_PATH=../target/riscv-unknown-kernel/release/tyche FW_PAYLOAD=y FW_PAYLOAD_PATH=../../xiangshan/Image_xiangshan CROSS_COMPILE=riscv64-linux-gnu- -j $(nproc) || exit 
cd ../../
python3 ./scripts/merge.py ./target/riscv-unknown-kernel/debug/anchor_xiangshan.img ./target/riscv-unknown-kernel/debug/tpm-driver_xiangshan.img ./tyche/opensbi-stage1/build/platform/generic/firmware/fw_payload.bin
