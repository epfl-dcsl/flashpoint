//! Physical Memory Protection

use crate::arch::Architecture;

use self::cfg::{L, NAPOT};

/// A range of memory
#[derive(Debug, Clone)]
pub struct Range {
    /// Start address (included)
    pub start: usize,
    /// End address (excluded)
    pub end: usize,
    /// Access rights
    pub perm: u8,
}

pub const fn range(start_end: (usize, usize), perm: u8) -> Range {
    let (start, end) = start_end;
    Range { start, end, perm }
}

/// PMP configuration bits
pub mod cfg {
    #![allow(dead_code)]

    /// Read access
    pub const R: u8 = 0b00000001;
    /// Write access
    pub const W: u8 = 0b00000010;
    /// Execute access
    pub const X: u8 = 0b00000100;
    /// Read and Write access
    pub const RW: u8 = R | W;
    /// Read, Write, and Execute access
    pub const RWX: u8 = R | W | X;
    /// No permissions
    pub const NO_PERMISSIONS: u8 = 0x0;

    /// Address is Top Of Range (TOP)
    pub const TOR: u8 = 0b00001000;
    /// Naturally aligned four-byte region
    pub const NA4: u8 = 0b00010000;
    /// Naturally aligned power of two
    pub const NAPOT: u8 = 0b00011000;
    /// Bit mask for the A attributes of pmpcfg
    pub const A_MASK: u8 = 0b00011000;

    /// Locked
    pub const L: u8 = 0b10000000;

    /// An inactive entry, ignored by the matching rules
    pub const INACTIVE: u8 = 0b00000000;

    /// Valid bits for pmpcfg (we currently configure only NAPOT encodings).
    pub const VALID_BITS: u8 = RWX | NAPOT | L;
}

/// Display PMPs, intended for debug purposes only
pub fn print_pmps() {
    use crate::arch::{Arch, Architecture};

    // Skip this function if log-level is too low
    if log::STATIC_MAX_LEVEL < log::Level::Info {
        return;
    }

    // We keep track of the previous address for TOR registers
    let mut prev_addr = 0;
    let hartid = Arch::read_mhartid();

    for i in 0..crate::PMP_ENTRIES {
        let addr = Arch::pmpaddr_csr_read(i);
        let cfg = (Arch::pmpcfg_csr_read(i) >> ((i % 8) * 8)) as u8;

        // Parse configuration
        let r = if cfg & 0b001 != 0 { 'R' } else { '_' };
        let w = if cfg & 0b010 != 0 { 'W' } else { '_' };
        let x = if cfg & 0b100 != 0 { 'X' } else { '_' };
        let l = if cfg & 0b10000000 != 0 { 'L' } else { ' ' };
        let a = (cfg >> 3) & 0b11;
        let mode = match a {
            0 => "OFF",
            1 => "TOR",
            2 => "NA4",
            3 => "NAPOT",
            _ => panic!("Unreacheable"),
        };

        // Compute start and end for each mode
        let (start, end) = match a {
            0 => {
                prev_addr = addr << 2;
                (addr << 2, 0)
            }
            1 => {
                let start = prev_addr;
                prev_addr = addr << 2;
                (start, addr << 2)
            }
            2 => {
                prev_addr = addr << 2;
                (addr, addr + 4)
            }
            3 => {
                let nb_ones = addr.trailing_ones();
                if nb_ones > u64::BITS - 2 {
                    (0, u64::MAX) // TODO: Not true when one bit to 0
                } else {
                    let start = (addr & !((1 << nb_ones) - 1)) << 2;
                    let size = 1 << (nb_ones + 3);
                    prev_addr = start;
                    (start as u64, start.saturating_add(size) as u64)
                }
            }
            _ => (addr, addr),
        };

        // Pretty print
        log::info!("hart {hartid} - PMP {i:2}  {start:16x} {end:16x} | {r}{w}{x}{l} {mode}");
    }
}
