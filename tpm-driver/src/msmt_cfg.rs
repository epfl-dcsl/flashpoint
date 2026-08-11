//! Cryptographic Measurement Configurations
//!
//! This module holds the addresses and sizes of the memory regions to measure.

pub const ANCHOR_ENTRY_TEXT_ADDRESS: usize = 0x80000000;
pub const ANCHOR_ENTRY_TEXT_SIZE: usize = 0x7d08;
pub const ANCHOR_RODATA_ADDRESS: usize = 0x80008000;
pub const ANCHOR_RODATA_SIZE: usize = 0x3b6f;
pub const ANCHOR_GOT_ADDRESS: usize = 0x8000bb70;
pub const ANCHOR_GOT_SIZE: usize = 0x58;

pub const TPM_DRV_ENTRY_TEXT_ADDRESS: usize = 0x80080000;
pub const TPM_DRV_ENTRY_TEXT_SIZE: usize = 0xbe14;
pub const TPM_DRV_RODATA_ADDRESS: usize = 0x8008c000;
pub const TPM_DRV_RODATA_SIZE: usize = 0x4257;
pub const TPM_DRV_GOT_ADDRESS: usize = 0x80090258;
pub const TPM_DRV_GOT_SIZE: usize = 0x68;