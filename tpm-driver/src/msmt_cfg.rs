//! Cryptographic Measurement Configurations
//!
//! This module holds the addresses and sizes of the memory regions to measure.

pub const ANCHOR_ENTRY_TEXT_ADDRESS: usize = 0x80000000;
pub const ANCHOR_ENTRY_TEXT_SIZE: usize = 0x6daa;
pub const ANCHOR_RODATA_ADDRESS: usize = 0x80007000;
pub const ANCHOR_RODATA_SIZE: usize = 0x3ebf;
pub const ANCHOR_GOT_ADDRESS: usize = 0x8000aec0;
pub const ANCHOR_GOT_SIZE: usize = 0x68;

pub const TPM_DRV_ENTRY_TEXT_ADDRESS: usize = 0x80080000;
pub const TPM_DRV_ENTRY_TEXT_SIZE: usize = 0xc64a;
pub const TPM_DRV_RODATA_ADDRESS: usize = 0x8008d000;
pub const TPM_DRV_RODATA_SIZE: usize = 0x46aa;
pub const TPM_DRV_GOT_ADDRESS: usize = 0x800916b0;
pub const TPM_DRV_GOT_SIZE: usize = 0x78;