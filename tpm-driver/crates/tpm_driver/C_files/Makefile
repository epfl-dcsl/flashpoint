SRC_DIR="./tpm_src/"
INCLUDE_DIR="./tpm_includes/"

archive:
	clang --target=riscv64 $(shell find $(SRC_DIR) -type f) -O0 -c -I $INCLUDE_DIR -fPIC
	ar crs libtpm_driver.a tpm_driver.o tcg_tpm.o sha.o
	ranlib libtpm_driver.a


clean: 
	rm *.o *.a
