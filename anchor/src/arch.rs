//! Architecture specific functions
//!
//! All direct interaction with RISC-V specific architecture features should live here. 

use crate::trap::{trap_handler, MCause};

// Those are only enabled when compiling against softcore
#[cfg(any(test, feature = "softcore"))]
use core::cell::RefCell;
#[cfg(any(test, feature = "softcore"))]
use softcore_asm_rv64::softcore_rv64::{config, new_core, Core};
#[cfg(any(test, feature = "softcore"))]
use std::thread_local;

// ———————————————————————————— Softcore Support ———————————————————————————— //

// Each thread gets its own copy of the core, this prevent tests using different threads inside a
// same process to share the same core.
#[cfg(any(test, feature = "softcore"))]
thread_local! {
    /// A software RISC-V core that emulates a real CPU.
    ///
    /// We use one core per thread to prevent interference among threads, such as when running
    /// `cargo test`. Therefore, the core lives in threat local storage and must be access using
    /// the `thread_local!` API.
    ///
    /// Usage:
    ///
    /// ```
    /// SOFT_CORE.with_borrow_mut(|core| {
    ///     // The `core` can be accessed within the closure
    ///     core.set(reg::X1, 0x42);
    ///     core.csrrw(reg::X0, csr::MSCRATCH, reg::X1).unwrap();
    /// });
    /// ```
    pub static SOFT_CORE: RefCell<Core> = {
        let mut core = new_core(config::U74);
        core.reset();
        RefCell::new(core)
    };
}

/// Unsafe placeholder function to make softcore-asm unsafe.
unsafe fn _unsafe_marker() {}

/// An assembly wrapper macro proxying to real assembly or softcore-rs.
///
/// The macro either emit standard `core::arch::asm` assembly, or calls into
/// `softcore_asm_rv64::asm` when running tests or verification.
macro_rules! soft_asm {
    ($($asm:tt)*) => {{
        #[cfg(not(any(test, feature = "softcore")))]
        core::arch::asm!(
            $($asm)*
        );

        #[cfg(any(test, feature = "softcore"))]
        softcore_asm_rv64::asm!(
            $($asm)*,
            softcore(SOFT_CORE.with_borrow_mut)
        );

        #[cfg(any(test, feature = "softcore"))]
        _unsafe_marker();
    }};
}

/// Creates a `naked_asm` function, either with real RISC-V assembly, or using softcore-asm if the
/// `softcore` feature is enabled.
macro_rules! soft_naked_asm {
    ($(#[link_section = $section:literal])? fn $name:ident, $($asm:tt)*) => {
        #[cfg(not(any(test, feature = "softcore")))]
        #[unsafe(naked)]

        $(#[link_section = $section])?
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() {
            core::arch::naked_asm!(
                $($asm)*
            );
        }

        #[cfg(any(test, feature = "softcore"))]
        pub fn $name() {
            unsafe {
                _unsafe_marker();

                softcore_asm_rv64::asm!(
                    $($asm)*,
                    softcore(SOFT_CORE.with_borrow_mut)
                );
            }
        }
    };
}

// ——————————————————————————— Assembly Wrappers ———————————————————————————— //

pub fn init() {
    // Set trap handler

    let handler = machine_trap_handler as *const () as usize;
    unsafe { write_mtvec(handler) };
    let mtvec = read_mtvec();
    assert_eq!(handler, mtvec, "Failed to set trap handler");

    log::debug!("Done setting up trap handler");
}

pub fn read_mhartid() -> usize {
    let mhartid: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mhartid",
            x = out(reg) mhartid);
    }
    mhartid
}

pub fn read_mstatus() -> usize {
    let mstatus: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mstatus",
            x = out(reg) mstatus);
    }
    mstatus
}

#[cfg(feature = "perf_counters")]
pub fn read_mcycle() -> u64 {
    let mcycle: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mcycle",
            x = out(reg) mcycle);
    }
    mcycle as u64
}

#[cfg(feature = "perf_counters")]
pub fn read_minstret() -> u64 {
    let minstret: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, minstret",
            x = out(reg) minstret);
    }
    minstret as u64
}

pub fn read_mcause() -> MCause {
    let mcause: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mcause",
            x = out(reg) mcause);
    }
    MCause::new(mcause)
}

pub fn read_mepc() -> usize {
    let mepc: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mepc",
            x = out(reg) mepc);
    }
    mepc
}

pub fn read_mtval() -> usize {
    let mtval: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mtval",
            x = out(reg) mtval);
    }
    mtval
}

pub unsafe fn write_mepc(mepc: usize) {
    soft_asm!(
        "csrw mepc, {x}",
        x = in(reg) mepc
    )
}

pub fn sfence_vma() {
    unsafe { 
        soft_asm!(
            "sfence.vma"
        ) 
    };
}

unsafe fn write_mtvec(value: usize) {
    soft_asm!(
        "csrw mtvec, {x}",
        x = in(reg) value
    )
}

fn read_mtvec() -> usize {
    let mtvec: usize;
    unsafe {
        soft_asm!(
            "csrr {x}, mtvec",
            x = out(reg) mtvec
        )
    }
    mtvec
}

/// Reset the CPU core registers before transfering control to the SM
pub fn reset_core_for_sm() {
    let mstatus: usize = 0xa00000000; // MXL=2 (RV64), SXL=2, UXL=2
    unsafe {
        soft_asm!(
            "csrw mstatus, {x}",
            x = in(reg) mstatus,
        );
        soft_asm!(
            "csrw mideleg, x0",
            "csrw medeleg, x0",
            "csrw mie, x0",
            "csrw mip, x0",
            "csrw mtvec, x0",
            "csrw mcause, x0",
            "csrw mtval, x0",
            "csrw mepc, x0",
            "csrw mscratch, x0",
        );
        #[cfg(not(feature = "xiangshan"))]
        soft_asm!(
            "csrw menvcfg, x0",
        )
    }
}

pub fn pmpaddr_csr_read(index: usize) -> u64 {
    let pmpaddr: u64;
    unsafe {
        match index {
            0 => soft_asm!("csrr {x}, pmpaddr0", x = out(reg) pmpaddr),
            1 => soft_asm!("csrr {x}, pmpaddr1", x = out(reg) pmpaddr),
            2 => soft_asm!("csrr {x}, pmpaddr2", x = out(reg) pmpaddr),
            3 => soft_asm!("csrr {x}, pmpaddr3", x = out(reg) pmpaddr),
            4 => soft_asm!("csrr {x}, pmpaddr4", x = out(reg) pmpaddr),
            5 => soft_asm!("csrr {x}, pmpaddr5", x = out(reg) pmpaddr),
            6 => soft_asm!("csrr {x}, pmpaddr6", x = out(reg) pmpaddr),
            7 => soft_asm!("csrr {x}, pmpaddr7", x = out(reg) pmpaddr),
            8 => soft_asm!("csrr {x}, pmpaddr8", x = out(reg) pmpaddr),
            9 => soft_asm!("csrr {x}, pmpaddr9", x = out(reg) pmpaddr),
            10 => soft_asm!("csrr {x}, pmpaddr10", x = out(reg) pmpaddr),
            11 => soft_asm!("csrr {x}, pmpaddr11", x = out(reg) pmpaddr),
            12 => soft_asm!("csrr {x}, pmpaddr12", x = out(reg) pmpaddr),
            13 => soft_asm!("csrr {x}, pmpaddr13", x = out(reg) pmpaddr),
            14 => soft_asm!("csrr {x}, pmpaddr14", x = out(reg) pmpaddr),
            15 => soft_asm!("csrr {x}, pmpaddr15", x = out(reg) pmpaddr),
            16 => soft_asm!("csrr {x}, pmpaddr16", x = out(reg) pmpaddr),
            17 => soft_asm!("csrr {x}, pmpaddr17", x = out(reg) pmpaddr),
            18 => soft_asm!("csrr {x}, pmpaddr18", x = out(reg) pmpaddr),
            19 => soft_asm!("csrr {x}, pmpaddr19", x = out(reg) pmpaddr),
            20 => soft_asm!("csrr {x}, pmpaddr20", x = out(reg) pmpaddr),
            21 => soft_asm!("csrr {x}, pmpaddr21", x = out(reg) pmpaddr),
            22 => soft_asm!("csrr {x}, pmpaddr22", x = out(reg) pmpaddr),
            23 => soft_asm!("csrr {x}, pmpaddr23", x = out(reg) pmpaddr),
            24 => soft_asm!("csrr {x}, pmpaddr24", x = out(reg) pmpaddr),
            25 => soft_asm!("csrr {x}, pmpaddr25", x = out(reg) pmpaddr),
            26 => soft_asm!("csrr {x}, pmpaddr26", x = out(reg) pmpaddr),
            27 => soft_asm!("csrr {x}, pmpaddr27", x = out(reg) pmpaddr),
            28 => soft_asm!("csrr {x}, pmpaddr28", x = out(reg) pmpaddr),
            29 => soft_asm!("csrr {x}, pmpaddr29", x = out(reg) pmpaddr),
            30 => soft_asm!("csrr {x}, pmpaddr30", x = out(reg) pmpaddr),
            31 => soft_asm!("csrr {x}, pmpaddr31", x = out(reg) pmpaddr),
            32 => soft_asm!("csrr {x}, pmpaddr32", x = out(reg) pmpaddr),
            33 => soft_asm!("csrr {x}, pmpaddr33", x = out(reg) pmpaddr),
            34 => soft_asm!("csrr {x}, pmpaddr34", x = out(reg) pmpaddr),
            35 => soft_asm!("csrr {x}, pmpaddr35", x = out(reg) pmpaddr),
            36 => soft_asm!("csrr {x}, pmpaddr36", x = out(reg) pmpaddr),
            37 => soft_asm!("csrr {x}, pmpaddr37", x = out(reg) pmpaddr),
            38 => soft_asm!("csrr {x}, pmpaddr38", x = out(reg) pmpaddr),
            39 => soft_asm!("csrr {x}, pmpaddr39", x = out(reg) pmpaddr),
            40 => soft_asm!("csrr {x}, pmpaddr40", x = out(reg) pmpaddr),
            41 => soft_asm!("csrr {x}, pmpaddr41", x = out(reg) pmpaddr),
            42 => soft_asm!("csrr {x}, pmpaddr42", x = out(reg) pmpaddr),
            43 => soft_asm!("csrr {x}, pmpaddr43", x = out(reg) pmpaddr),
            44 => soft_asm!("csrr {x}, pmpaddr44", x = out(reg) pmpaddr),
            45 => soft_asm!("csrr {x}, pmpaddr45", x = out(reg) pmpaddr),
            46 => soft_asm!("csrr {x}, pmpaddr46", x = out(reg) pmpaddr),
            47 => soft_asm!("csrr {x}, pmpaddr47", x = out(reg) pmpaddr),
            48 => soft_asm!("csrr {x}, pmpaddr48", x = out(reg) pmpaddr),
            49 => soft_asm!("csrr {x}, pmpaddr49", x = out(reg) pmpaddr),
            50 => soft_asm!("csrr {x}, pmpaddr50", x = out(reg) pmpaddr),
            51 => soft_asm!("csrr {x}, pmpaddr51", x = out(reg) pmpaddr),
            52 => soft_asm!("csrr {x}, pmpaddr52", x = out(reg) pmpaddr),
            53 => soft_asm!("csrr {x}, pmpaddr53", x = out(reg) pmpaddr),
            54 => soft_asm!("csrr {x}, pmpaddr54", x = out(reg) pmpaddr),
            55 => soft_asm!("csrr {x}, pmpaddr55", x = out(reg) pmpaddr),
            56 => soft_asm!("csrr {x}, pmpaddr56", x = out(reg) pmpaddr),
            57 => soft_asm!("csrr {x}, pmpaddr57", x = out(reg) pmpaddr),
            58 => soft_asm!("csrr {x}, pmpaddr58", x = out(reg) pmpaddr),
            59 => soft_asm!("csrr {x}, pmpaddr59", x = out(reg) pmpaddr),
            60 => soft_asm!("csrr {x}, pmpaddr60", x = out(reg) pmpaddr),
            61 => soft_asm!("csrr {x}, pmpaddr61", x = out(reg) pmpaddr),
            62 => soft_asm!("csrr {x}, pmpaddr62", x = out(reg) pmpaddr),
            63 => soft_asm!("csrr {x}, pmpaddr63", x = out(reg) pmpaddr),
            _ => pmpaddr = 0, // Default case when index is not
        }
    }
    pmpaddr
}

pub fn pmpaddr_csr_write(index: usize, pmpaddr: usize) {
    unsafe {
        match index {
            0 => soft_asm!("csrw pmpaddr0, {x}", x = in(reg) pmpaddr),
            1 => soft_asm!("csrw pmpaddr1, {x}", x = in(reg) pmpaddr),
            2 => soft_asm!("csrw pmpaddr2, {x}", x = in(reg) pmpaddr),
            3 => soft_asm!("csrw pmpaddr3, {x}", x = in(reg) pmpaddr),
            4 => soft_asm!("csrw pmpaddr4, {x}", x = in(reg) pmpaddr),
            5 => soft_asm!("csrw pmpaddr5, {x}", x = in(reg) pmpaddr),
            6 => soft_asm!("csrw pmpaddr6, {x}", x = in(reg) pmpaddr),
            7 => soft_asm!("csrw pmpaddr7, {x}", x = in(reg) pmpaddr),
            8 => soft_asm!("csrw pmpaddr8, {x}", x = in(reg) pmpaddr),
            9 => soft_asm!("csrw pmpaddr9, {x}", x = in(reg) pmpaddr),
            10 => soft_asm!("csrw pmpaddr10, {x}", x = in(reg) pmpaddr),
            11 => soft_asm!("csrw pmpaddr11, {x}", x = in(reg) pmpaddr),
            12 => soft_asm!("csrw pmpaddr12, {x}", x = in(reg) pmpaddr),
            13 => soft_asm!("csrw pmpaddr13, {x}", x = in(reg) pmpaddr),
            14 => soft_asm!("csrw pmpaddr14, {x}", x = in(reg) pmpaddr),
            15 => soft_asm!("csrw pmpaddr15, {x}", x = in(reg) pmpaddr),
            16 => soft_asm!("csrw pmpaddr16, {x}", x = in(reg) pmpaddr),
            17 => soft_asm!("csrw pmpaddr17, {x}", x = in(reg) pmpaddr),
            18 => soft_asm!("csrw pmpaddr18, {x}", x = in(reg) pmpaddr),
            19 => soft_asm!("csrw pmpaddr19, {x}", x = in(reg) pmpaddr),
            20 => soft_asm!("csrw pmpaddr20, {x}", x = in(reg) pmpaddr),
            21 => soft_asm!("csrw pmpaddr21, {x}", x = in(reg) pmpaddr),
            22 => soft_asm!("csrw pmpaddr22, {x}", x = in(reg) pmpaddr),
            23 => soft_asm!("csrw pmpaddr23, {x}", x = in(reg) pmpaddr),
            24 => soft_asm!("csrw pmpaddr24, {x}", x = in(reg) pmpaddr),
            25 => soft_asm!("csrw pmpaddr25, {x}", x = in(reg) pmpaddr),
            26 => soft_asm!("csrw pmpaddr26, {x}", x = in(reg) pmpaddr),
            27 => soft_asm!("csrw pmpaddr27, {x}", x = in(reg) pmpaddr),
            28 => soft_asm!("csrw pmpaddr28, {x}", x = in(reg) pmpaddr),
            29 => soft_asm!("csrw pmpaddr29, {x}", x = in(reg) pmpaddr),
            30 => soft_asm!("csrw pmpaddr30, {x}", x = in(reg) pmpaddr),
            31 => soft_asm!("csrw pmpaddr31, {x}", x = in(reg) pmpaddr),
            32 => soft_asm!("csrw pmpaddr32, {x}", x = in(reg) pmpaddr),
            33 => soft_asm!("csrw pmpaddr33, {x}", x = in(reg) pmpaddr),
            34 => soft_asm!("csrw pmpaddr34, {x}", x = in(reg) pmpaddr),
            35 => soft_asm!("csrw pmpaddr35, {x}", x = in(reg) pmpaddr),
            36 => soft_asm!("csrw pmpaddr36, {x}", x = in(reg) pmpaddr),
            37 => soft_asm!("csrw pmpaddr37, {x}", x = in(reg) pmpaddr),
            38 => soft_asm!("csrw pmpaddr38, {x}", x = in(reg) pmpaddr),
            39 => soft_asm!("csrw pmpaddr39, {x}", x = in(reg) pmpaddr),
            40 => soft_asm!("csrw pmpaddr40, {x}", x = in(reg) pmpaddr),
            41 => soft_asm!("csrw pmpaddr41, {x}", x = in(reg) pmpaddr),
            42 => soft_asm!("csrw pmpaddr42, {x}", x = in(reg) pmpaddr),
            43 => soft_asm!("csrw pmpaddr43, {x}", x = in(reg) pmpaddr),
            44 => soft_asm!("csrw pmpaddr44, {x}", x = in(reg) pmpaddr),
            45 => soft_asm!("csrw pmpaddr45, {x}", x = in(reg) pmpaddr),
            46 => soft_asm!("csrw pmpaddr46, {x}", x = in(reg) pmpaddr),
            47 => soft_asm!("csrw pmpaddr47, {x}", x = in(reg) pmpaddr),
            48 => soft_asm!("csrw pmpaddr48, {x}", x = in(reg) pmpaddr),
            49 => soft_asm!("csrw pmpaddr49, {x}", x = in(reg) pmpaddr),
            50 => soft_asm!("csrw pmpaddr50, {x}", x = in(reg) pmpaddr),
            51 => soft_asm!("csrw pmpaddr51, {x}", x = in(reg) pmpaddr),
            52 => soft_asm!("csrw pmpaddr52, {x}", x = in(reg) pmpaddr),
            53 => soft_asm!("csrw pmpaddr53, {x}", x = in(reg) pmpaddr),
            54 => soft_asm!("csrw pmpaddr54, {x}", x = in(reg) pmpaddr),
            55 => soft_asm!("csrw pmpaddr55, {x}", x = in(reg) pmpaddr),
            56 => soft_asm!("csrw pmpaddr56, {x}", x = in(reg) pmpaddr),
            57 => soft_asm!("csrw pmpaddr57, {x}", x = in(reg) pmpaddr),
            58 => soft_asm!("csrw pmpaddr58, {x}", x = in(reg) pmpaddr),
            59 => soft_asm!("csrw pmpaddr59, {x}", x = in(reg) pmpaddr),
            60 => soft_asm!("csrw pmpaddr60, {x}", x = in(reg) pmpaddr),
            61 => soft_asm!("csrw pmpaddr61, {x}", x = in(reg) pmpaddr),
            62 => soft_asm!("csrw pmpaddr62, {x}", x = in(reg) pmpaddr),
            63 => soft_asm!("csrw pmpaddr63, {x}", x = in(reg) pmpaddr),
            _ => (), // Default case when index is not matched
        }
    }
}

pub fn pmpcfg_csr_read(index: usize) -> u64 {
    //The following code supports 64 PMP entries regardless of the value of PMP_ENTRIES. The
    //check in pmp_read is what will ensure that the index is valid. The following code should
    //execute only if that check passes, and thus should not access an index which shouldn't be supported.

    let pmpcfg: u64;
    //log::info!("index {}", index);
    match index {
        0..=7 => {
            /*log::info!("Matched 0");*/
            unsafe {
                soft_asm!("csrr {x}, pmpcfg0", x = out(reg) pmpcfg);
            }
        }
        8..=15 => unsafe {
            soft_asm!("csrr {x}, pmpcfg2", x = out(reg) pmpcfg);
        },
        16..=23 => unsafe {
            soft_asm!("csrr {x}, pmpcfg4", x = out(reg) pmpcfg);
        },
        24..=31 => unsafe {
            soft_asm!("csrr {x}, pmpcfg6", x = out(reg) pmpcfg);
        },
        32..=39 => unsafe {
            soft_asm!("csrr {x}, pmpcfg8", x = out(reg) pmpcfg);
        },
        40..=47 => unsafe {
            soft_asm!("csrr {x}, pmpcfg10", x = out(reg) pmpcfg);
        },
        48..=55 => unsafe {
            soft_asm!("csrr {x}, pmpcfg12", x = out(reg) pmpcfg);
        },
        56..=63 => unsafe {
            soft_asm!("csrr {x}, pmpcfg14", x = out(reg) pmpcfg);
        },
        _ => {
            log::info!("Matched None");
            pmpcfg = 0;
        }
    }
    pmpcfg
}

pub fn pmpcfg_csr_write(index: usize, pmpcfg: usize) {
    //log::info!("index {x}", x= index);
    match index {
        0..=7 => {
            /*log::info!("Matched 0 writing: {:x}", pmpcfg);*/
            unsafe {
                soft_asm!("csrw pmpcfg0, {x}", x= in(reg) pmpcfg);
            }
        }
        8..=15 => unsafe {
            soft_asm!("csrw pmpcfg2, {x}", x= in(reg) pmpcfg);
        },
        16..=23 => unsafe {
            soft_asm!("csrw pmpcfg4, {x}", x= in(reg) pmpcfg);
        },
        24..=31 => unsafe {
            soft_asm!("csrw pmpcfg6, {x}", x= in(reg) pmpcfg);
        },
        32..=39 => unsafe {
            soft_asm!("csrw pmpcfg8, {x}", x= in(reg) pmpcfg);
        },
        40..=47 => unsafe {
            soft_asm!("csrw pmpcfg10, {x}", x= in(reg) pmpcfg);
        },
        48..=55 => unsafe {
            soft_asm!("csrw pmpcfg12, {x}", x= in(reg) pmpcfg);
        },
        56..=63 => unsafe {
            soft_asm!("csrw pmpcfg14, {x}", x= in(reg) pmpcfg);
        },
        _ => {
            log::info!("Matched None");
        }
    }
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub struct RegisterArguments {
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
}

// —————————————————————————————————— CSRs —————————————————————————————————— //

/// Module with the CSR registers indexes.
pub mod csr {
    #![allow(unused)]

    // Machine mode CSRs
    pub const MSTATUS: u64 = 0x300;
    pub const MISA: u64 = 0x301;
    pub const MEDELEG: u64 = 0x302;
    pub const MIDELEG: u64 = 0x303;
    pub const MIE: u64 = 0x304;
    pub const MTVEC: u64 = 0x305;
    pub const MCOUNTEREN: u64 = 0x306;
    pub const MENVCFG: u64 = 0x30A;
    pub const MCOUNTINHIBIT: u64 = 0x320;
    pub const MHPMEVENT3: u64 = 0x323;
    pub const MHPMEVENT31: u64 = 0x33F;
    pub const MSCRATCH: u64 = 0x340;
    pub const MEPC: u64 = 0x341;
    pub const MCAUSE: u64 = 0x342;
    pub const MTVAL: u64 = 0x343;
    pub const MIP: u64 = 0x344;
    pub const MTINST: u64 = 0x34A;
    pub const MTVAL2: u64 = 0x34B;
    pub const PMPCFG0: u64 = 0x3A0;
    pub const PMPCFG15: u64 = 0x3AF;
    pub const PMPADDR0: u64 = 0x3B0;
    pub const PMPADDR63: u64 = 0x3EF;
    pub const MSECCFG: u64 = 0x747;
    pub const TSELECT: u64 = 0x7A0;
    pub const TDATA1: u64 = 0x7A1;
    pub const TDATA2: u64 = 0x7A2;
    pub const TDATA3: u64 = 0x7A3;
    pub const MCONTEXT: u64 = 0x7A8;
    pub const DCSR: u64 = 0x7B0;
    pub const DPC: u64 = 0x7B1;
    pub const DSCRATCH0: u64 = 0x7B2;
    pub const DSCRATCH1: u64 = 0x7B3;
    pub const MCYCLE: u64 = 0xB00;
    pub const MINSTRET: u64 = 0xB02;
    pub const MHPMCOUNTER3: u64 = 0xB03;
    pub const MHPMCOUNTER31: u64 = 0xB1F;
    pub const CYCLE: u64 = 0xC00;
    pub const TIME: u64 = 0xC01;
    pub const INSTRET: u64 = 0xC02;
    pub const VL: u64 = 0xC20;
    pub const VTYPE: u64 = 0xC21;
    pub const VLENB: u64 = 0xC22;
    pub const MVENDORID: u64 = 0xF11;
    pub const MARCHID: u64 = 0xF12;
    pub const MIMPID: u64 = 0xF13;
    pub const MHARTID: u64 = 0xF14;
    pub const MCONFIGPTR: u64 = 0xF15;

    // Supervisor mode CSRs
    pub const SSTATUS: u64 = 0x100;
    pub const SIE: u64 = 0x104;
    pub const STVEC: u64 = 0x105;
    pub const SCOUNTEREN: u64 = 0x106;
    pub const SENVCFG: u64 = 0x10A;
    pub const SSCRATCH: u64 = 0x140;
    pub const SEPC: u64 = 0x141;
    pub const SCAUSE: u64 = 0x142;
    pub const STVAL: u64 = 0x143;
    pub const SIP: u64 = 0x144;
    pub const STIMECMP: u64 = 0x14D;
    pub const SATP: u64 = 0x180;
    pub const SCONTEXT: u64 = 0x5A8;

    // Hypervisor and Virtual Supervisor CSRs
    pub const VSSTATUS: u64 = 0x200;
    pub const VSIE: u64 = 0x204;
    pub const VSTVEC: u64 = 0x205;
    pub const VSSCRATCH: u64 = 0x240;
    pub const VSEPC: u64 = 0x241;
    pub const VSCAUSE: u64 = 0x242;
    pub const VSTVAL: u64 = 0x243;
    pub const VSIP: u64 = 0x244;
    pub const VSATP: u64 = 0x280;
    pub const HSTATUS: u64 = 0x600;
    pub const HEDELEG: u64 = 0x602;
    pub const HIDELEG: u64 = 0x603;
    pub const HIE: u64 = 0x604;
    pub const HTIMEDELTA: u64 = 0x605;
    pub const HCOUNTEREN: u64 = 0x606;
    pub const HGEIE: u64 = 0x607;
    pub const HGEIP: u64 = 0xE12;
    pub const HENVCFG: u64 = 0x60A;
    pub const HTVAL: u64 = 0x643;
    pub const HIP: u64 = 0x644;
    pub const HVIP: u64 = 0x645;
    pub const HTINST: u64 = 0x64A;
    pub const HGATP: u64 = 0x680;

    // Vector extension CSRs
    pub const VSTART: u64 = 0x8;
    pub const VXSAT: u64 = 0x9;
    pub const VXRM: u64 = 0xA;
    pub const VCSR: u64 = 0xF;

    // Crypto extension CSRs
    pub const SEED: u64 = 0x15;
}

// ————————————————————————— Platform specific code ————————————————————————— //

// M-mode trap handler
// Saves register state - calls trap_handler - restores register state - mret to intended mode.

soft_naked_asm! {
    fn machine_trap_handler,
    "csrrw sp, mscratch, sp
    addi sp, sp, -34*8
    sd zero, 0*8(sp)    // WHY?
    sd ra, 1*8(sp)
    sd zero, 2*8(sp)    //uninitialised sp
    sd gp, 3*8(sp)
    sd tp, 4*8(sp)
    sd t0, 5*8(sp)
    sd t1, 6*8(sp)
    sd t2, 7*8(sp)
    sd s0, 8*8(sp)
    sd s1, 9*8(sp)
    sd a0, 10*8(sp)
    sd a1, 11*8(sp)
    sd a2, 12*8(sp)
    sd a3, 13*8(sp)
    sd a4, 14*8(sp)
    sd a5, 15*8(sp)
    sd a6, 16*8(sp)
    sd a7, 17*8(sp)
    sd s2, 18*8(sp)
    sd s3, 19*8(sp)
    sd s4, 20*8(sp)
    sd s5, 21*8(sp)
    sd s6, 22*8(sp)
    sd s7, 23*8(sp)
    sd s8, 24*8(sp)
    sd s9, 25*8(sp)
    sd s10, 26*8(sp)
    sd s11, 27*8(sp)
    sd t3, 28*8(sp)
    sd t4, 29*8(sp)
    sd t5, 30*8(sp)
    sd t6, 31*8(sp)
    mv a0, sp      //arg to trap_handler

    // We call into the trap handler there.
    // Softcore requires use to annotate the call with the expected ABI.
    //
    // #[abi(\"C\", 0)]
    call {trap_handler}

    ld zero, 0*8(sp)
    ld ra, 1*8(sp)
    ld gp, 3*8(sp)
    ld tp, 4*8(sp)
    ld t0, 5*8(sp)
    ld t1, 6*8(sp)
    ld t2, 7*8(sp)
    ld s0, 8*8(sp)
    ld s1, 9*8(sp)
    ld a0, 10*8(sp)
    ld a1, 11*8(sp)
    ld a2, 12*8(sp)
    ld a3, 13*8(sp)
    ld a4, 14*8(sp)
    ld a5, 15*8(sp)
    ld a6, 16*8(sp)
    ld a7, 17*8(sp)
    ld s2, 18*8(sp)
    ld s3, 19*8(sp)
    ld s4, 20*8(sp)
    ld s5, 21*8(sp)
    ld s6, 22*8(sp)
    ld s7, 23*8(sp)
    ld s8, 24*8(sp)
    ld s9, 25*8(sp)
    ld s10, 26*8(sp)
    ld s11, 27*8(sp)
    ld t3, 28*8(sp)
    ld t4, 29*8(sp)
    ld t5, 30*8(sp)
    ld t6, 31*8(sp)
    addi sp, sp, 34*8
    csrrw sp, mscratch, sp
    mret",
    trap_handler = sym trap_handler
}

soft_naked_asm! {
    #[link_section = ".entry_point"]
    fn _start,

    "csrr t0, mhartid",
    "slli t0, t0, 3",    // to index into STACK_ADDRESS
    "la t1, {stack}",
    "add t1, t1, t0",
    "ld sp, 0(t1)",      // After this the stack is initialized
    "csrw mscratch, sp", // save sp in mscratch -- in case the anchor traps

    // Softcore requires to specify the ABI when calling a function from assembly
    "// #[abi(\"C-unwind\", 5, u64)]",
    "call {anchor_main}",
    // Returns hartid in a0

    // // Find the return values
    // "csrr t0, mhartid",
    "slli t0, a0, 5",    // t0 = t0 * 32, where 32 = 8 * 4, so arrays of four 8 bytes elements
    "la t1, {ret_vals}",
    "add t1, t1, t0",
    "ld a0,  0(t1)",
    "ld a1,  8(t1)",
    "ld t0, 16(t1)",

     // Reset all registers except a0 (x10), a1 (x11), and t0 (x5).
     "mv ra, zero",   // x1
     "mv sp, zero",   // x2
     "mv gp, zero",   // x3
     "mv tp, zero",   // x4
     "mv t1, zero",   // x6
     "mv t2, zero",   // x7
     "mv s0, zero",   // x8
     "mv s1, zero",   // x9
     "mv a2, zero",   // x12
     "mv a3, zero",   // x13
     "mv a4, zero",   // x14
     "mv a5, zero",   // x15
     "mv a6, zero",   // x16
     "mv a7, zero",   // x17
     "mv s2, zero",   // x18
     "mv s3, zero",   // x19
     "mv s4, zero",   // x20
     "mv s5, zero",   // x21
     "mv s6, zero",   // x22
     "mv s7, zero",   // x23
     "mv s8, zero",   // x24
     "mv s9, zero",   // x25
     "mv s10, zero",  // x26
     "mv s11, zero",  // x27
     "mv t3, zero",   // x28
     "mv t4, zero",   // x29
     "mv t5, zero",   // x30
     "mv t6, zero",   // x31

    // We use a raw .word when compiling for RISC-V as the compiler is not aware
    // of the flashpoint instructions.
    "// #[replace_with(\"anchor.exit t0, t1\")]",
    ".word 0x0062802b",

    stack = sym crate::STACK_ADDRESS,
    ret_vals = sym crate::RETURN_VALUES,
    anchor_main = sym crate::anchor_main,
}
