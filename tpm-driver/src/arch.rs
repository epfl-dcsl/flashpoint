//! Architecture specific functions

use core::arch::{asm, naked_asm};

pub static FIRMWARE_STACK_POINTER: usize = 0x800f0000;

use crate::trap::{trap_handler, MCause};

/// Export the current architecture.
/// For now, only bare-metal is supported
pub type Arch = Metal;

/// Architecture abstraction layer.
pub trait Architecture {
    fn init();
    fn read_mstatus() -> usize;
    fn read_mcause() -> MCause;
    fn read_mepc() -> usize;
    fn read_mtval() -> usize;
    fn read_pmpcfg0() -> usize; 
    fn read_pmpcfg2() -> usize;     // TODO: Make it scale 
    fn read_mcycle() -> u64;
    fn read_minstret() -> u64;
    unsafe fn write_mepc(mepc: usize);
    unsafe fn mret();
    unsafe fn ecall();
    unsafe fn sfence_vma();
    unsafe fn enter_anchor(exit_address: usize, exit_region_size: usize, entry_minstret: usize, entry_mcycle: usize) -> !;
    unsafe fn prepare_platform_for_oem_firmware(hart_id: usize, device_tree_blob_addr: usize);
    fn pmpaddr_csr_read(index: usize) -> u64;
    fn pmpcfg_csr_read(index: usize) -> u64;
    fn read_mhartid() -> usize;
}

// ——————————————————————————————— Bare Metal ——————————————————————————————— //

/// Bare metal RISC-V runtime.
pub struct Metal {}

impl Architecture for Metal {
    fn init() {
        // Set trap handler
        unsafe {
            asm!("csrw mscratch, {}", in(reg) FIRMWARE_STACK_POINTER);
        }
        let handler = machine_trap_handler as usize;
        unsafe { write_mtvec(handler) };
        let mtvec = read_mtvec();
        assert_eq!(handler, mtvec, "Failed to set trap handler");
        //log::info!("Done setting up trap handler");
    }

    fn read_mhartid() -> usize {
        let mhartid: usize;
        unsafe {
            asm!(
                "csrr {x}, mhartid",
                x = out(reg) mhartid);
        }
        mhartid
    }

    fn read_mstatus() -> usize {
        let mstatus: usize;
        unsafe {
            asm!(
                "csrr {x}, mstatus",
                x = out(reg) mstatus);
        }
        return mstatus;
    }

    fn read_mcause() -> MCause {
        log::info!("Reading mcause!");
        let mcause: usize;
        unsafe {
            asm!(
                "csrr {x}, mcause",
                x = out(reg) mcause);
        }
        //log::info!("Done reading mcause!");
        return MCause::new(mcause);
    }

    fn read_mepc() -> usize {
        let mepc: usize;
        unsafe {
            asm!(
                "csrr {x}, mepc",
                x = out(reg) mepc);
        }
        return mepc;
    }

    fn read_pmpcfg0() -> usize {
        let pmpcfg0: usize;
        unsafe {
            asm!(
                "csrr {x}, pmpcfg0",
                x = out(reg) pmpcfg0);
        }
        return pmpcfg0;
    }

    fn read_pmpcfg2() -> usize {
        let pmpcfg2: usize;
        unsafe {
            asm!(
                "csrr {x}, pmpcfg2",
                x = out(reg) pmpcfg2);
        }
        return pmpcfg2;
    }

    fn read_mtval() -> usize {
        let mtval: usize;
        unsafe {
            asm!(
                "csrr {x}, mtval",
                x = out(reg) mtval);
        }
        return mtval;
    }

    fn read_mcycle() -> u64 {
        let mcycle: usize;
        unsafe {
            asm!(
                "csrr {x}, mcycle",
                x = out(reg) mcycle);
        }
        mcycle as u64
    }

    fn read_minstret() -> u64 {
        let minstret: usize;
        unsafe {
            asm!(
                "csrr {x}, minstret",
                x = out(reg) minstret);
        }
        minstret as u64
    }

    unsafe fn write_mepc(mepc: usize) {
        asm!(
            "csrw mepc, {x}",
            x = in(reg) mepc
        )
    }

    unsafe fn mret() {
        asm!("mret")
    }

    unsafe fn ecall() {
        asm!("ecall")
    }

    unsafe fn sfence_vma() {
        asm!("sfence.vma");
    }

    unsafe fn enter_anchor(exit_address: usize, exit_region_size: usize, entry_minstret: usize, entry_mcycle: usize) -> ! {
        asm!(
            "mv t0, {addr}",
            "mv t1, {size}",   
            addr = in(reg) exit_address,
            size = in(reg) exit_region_size,
            out("t0") _,
            out("t1") _,
            out("a0") _,
            out("a1") _,
            out("a2") _,
            out("a3") _,
        );
        asm!(
            ".word 0x0002902b",      
            options(noreturn)
        );
    }

    unsafe fn prepare_platform_for_oem_firmware(hart_id: usize, device_tree_blob_addr: usize) {
        asm!(
            "mv a0, {hart_id}", //x10
            "mv a1, {dtb_addr}", //x11
            // "mv x1, x0",
            // "mv x2, x0",
            // "mv x3, x0",
            // "mv x4, x0",
            // "mv x5, x0",
            // "mv x6, x0",
            // "mv x7, x0",
            // "mv x8, x0",
            // "mv x9, x0",
            // "mv x12, x0",
            // "mv x13, x0",
            // "mv x14, x0",
            // "mv x15, x0",
            // "mv x16, x0",
            // "mv x17, x0",
            // "mv x18, x0",
            // "mv x19, x0",
            // "mv x20, x0",
            // "mv x21, x0",
            // "mv x22, x0",
            // "mv x23, x0",
            // "mv x24, x0",
            // "mv x25, x0",
            // "mv x26, x0",
            // "mv x27, x0",
            // "mv x28, x0",
            // "mv x29, x0",
            // "mv x30, x0",
            // "mv x31, x0",
            //"mv sp, x0",
            hart_id = in(reg) hart_id,
            dtb_addr = in(reg) device_tree_blob_addr,
        );
    }

    // unsafe fn init_pmps() {
    //     for n in 0..PMP_ENTRIES {
    //         pmpaddr_csr_write(n, 0);
    //         pmpcfg_csr_write(n, 0); //Note: This only works because the frozen_pmp entry we have
    //                                 //has pmpcfg = 0. If that wasn't the case, clearing would
    //                                 //require fine-grained writes to pmpcfg.
    //     }
    // }

    fn pmpaddr_csr_read(index: usize) -> u64 {
        let pmpaddr: u64;
        unsafe {
            match index {
                0 => asm!("csrr {x}, pmpaddr0", x = out(reg) pmpaddr),
                1 => asm!("csrr {x}, pmpaddr1", x = out(reg) pmpaddr),
                2 => asm!("csrr {x}, pmpaddr2", x = out(reg) pmpaddr),
                3 => asm!("csrr {x}, pmpaddr3", x = out(reg) pmpaddr),
                4 => asm!("csrr {x}, pmpaddr4", x = out(reg) pmpaddr),
                5 => asm!("csrr {x}, pmpaddr5", x = out(reg) pmpaddr),
                6 => asm!("csrr {x}, pmpaddr6", x = out(reg) pmpaddr),
                7 => asm!("csrr {x}, pmpaddr7", x = out(reg) pmpaddr),
                8 => asm!("csrr {x}, pmpaddr8", x = out(reg) pmpaddr),
                9 => asm!("csrr {x}, pmpaddr9", x = out(reg) pmpaddr),
                10 => asm!("csrr {x}, pmpaddr10", x = out(reg) pmpaddr),
                11 => asm!("csrr {x}, pmpaddr11", x = out(reg) pmpaddr),
                12 => asm!("csrr {x}, pmpaddr12", x = out(reg) pmpaddr),
                13 => asm!("csrr {x}, pmpaddr13", x = out(reg) pmpaddr),
                14 => asm!("csrr {x}, pmpaddr14", x = out(reg) pmpaddr),
                15 => asm!("csrr {x}, pmpaddr15", x = out(reg) pmpaddr),
                16 => asm!("csrr {x}, pmpaddr16", x = out(reg) pmpaddr),
                17 => asm!("csrr {x}, pmpaddr17", x = out(reg) pmpaddr),
                18 => asm!("csrr {x}, pmpaddr18", x = out(reg) pmpaddr),
                19 => asm!("csrr {x}, pmpaddr19", x = out(reg) pmpaddr),
                20 => asm!("csrr {x}, pmpaddr20", x = out(reg) pmpaddr),
                21 => asm!("csrr {x}, pmpaddr21", x = out(reg) pmpaddr),
                22 => asm!("csrr {x}, pmpaddr22", x = out(reg) pmpaddr),
                23 => asm!("csrr {x}, pmpaddr23", x = out(reg) pmpaddr),
                24 => asm!("csrr {x}, pmpaddr24", x = out(reg) pmpaddr),
                25 => asm!("csrr {x}, pmpaddr25", x = out(reg) pmpaddr),
                26 => asm!("csrr {x}, pmpaddr26", x = out(reg) pmpaddr),
                27 => asm!("csrr {x}, pmpaddr27", x = out(reg) pmpaddr),
                28 => asm!("csrr {x}, pmpaddr28", x = out(reg) pmpaddr),
                29 => asm!("csrr {x}, pmpaddr29", x = out(reg) pmpaddr),
                30 => asm!("csrr {x}, pmpaddr30", x = out(reg) pmpaddr),
                31 => asm!("csrr {x}, pmpaddr31", x = out(reg) pmpaddr),
                32 => asm!("csrr {x}, pmpaddr32", x = out(reg) pmpaddr),
                33 => asm!("csrr {x}, pmpaddr33", x = out(reg) pmpaddr),
                34 => asm!("csrr {x}, pmpaddr34", x = out(reg) pmpaddr),
                35 => asm!("csrr {x}, pmpaddr35", x = out(reg) pmpaddr),
                36 => asm!("csrr {x}, pmpaddr36", x = out(reg) pmpaddr),
                37 => asm!("csrr {x}, pmpaddr37", x = out(reg) pmpaddr),
                38 => asm!("csrr {x}, pmpaddr38", x = out(reg) pmpaddr),
                39 => asm!("csrr {x}, pmpaddr39", x = out(reg) pmpaddr),
                40 => asm!("csrr {x}, pmpaddr40", x = out(reg) pmpaddr),
                41 => asm!("csrr {x}, pmpaddr41", x = out(reg) pmpaddr),
                42 => asm!("csrr {x}, pmpaddr42", x = out(reg) pmpaddr),
                43 => asm!("csrr {x}, pmpaddr43", x = out(reg) pmpaddr),
                44 => asm!("csrr {x}, pmpaddr44", x = out(reg) pmpaddr),
                45 => asm!("csrr {x}, pmpaddr45", x = out(reg) pmpaddr),
                46 => asm!("csrr {x}, pmpaddr46", x = out(reg) pmpaddr),
                47 => asm!("csrr {x}, pmpaddr47", x = out(reg) pmpaddr),
                48 => asm!("csrr {x}, pmpaddr48", x = out(reg) pmpaddr),
                49 => asm!("csrr {x}, pmpaddr49", x = out(reg) pmpaddr),
                50 => asm!("csrr {x}, pmpaddr50", x = out(reg) pmpaddr),
                51 => asm!("csrr {x}, pmpaddr51", x = out(reg) pmpaddr),
                52 => asm!("csrr {x}, pmpaddr52", x = out(reg) pmpaddr),
                53 => asm!("csrr {x}, pmpaddr53", x = out(reg) pmpaddr),
                54 => asm!("csrr {x}, pmpaddr54", x = out(reg) pmpaddr),
                55 => asm!("csrr {x}, pmpaddr55", x = out(reg) pmpaddr),
                56 => asm!("csrr {x}, pmpaddr56", x = out(reg) pmpaddr),
                57 => asm!("csrr {x}, pmpaddr57", x = out(reg) pmpaddr),
                58 => asm!("csrr {x}, pmpaddr58", x = out(reg) pmpaddr),
                59 => asm!("csrr {x}, pmpaddr59", x = out(reg) pmpaddr),
                60 => asm!("csrr {x}, pmpaddr60", x = out(reg) pmpaddr),
                61 => asm!("csrr {x}, pmpaddr61", x = out(reg) pmpaddr),
                62 => asm!("csrr {x}, pmpaddr62", x = out(reg) pmpaddr),
                63 => asm!("csrr {x}, pmpaddr63", x = out(reg) pmpaddr),
                _ => pmpaddr = 0, // Default case when index is not
            }
        }
        pmpaddr
    }

    fn pmpcfg_csr_read(index: usize) -> u64 {
        //The following code supports 64 PMP entries regardless of the value of PMP_ENTRIES. The
        //check in pmp_read is what will ensure that the index is valid. The following code should
        //execute only if that check passes, and thus should not access an index which shouldn't be supported.
    
        let pmpcfg: u64;
        //log::info!("index {}", index);
        match index {
            0..=7 => {
                /*log::info!("Matched 0");*/
                unsafe {
                    asm!("csrr {x}, pmpcfg0", x = out(reg) pmpcfg);
                }
            }
            8..=15 => unsafe {
                asm!("csrr {x}, pmpcfg2", x = out(reg) pmpcfg);
            },
            16..=23 => unsafe {
                asm!("csrr {x}, pmpcfg4", x = out(reg) pmpcfg);
            },
            24..=31 => unsafe {
                asm!("csrr {x}, pmpcfg6", x = out(reg) pmpcfg);
            },
            32..=39 => unsafe {
                asm!("csrr {x}, pmpcfg8", x = out(reg) pmpcfg);
            },
            40..=47 => unsafe {
                asm!("csrr {x}, pmpcfg10", x = out(reg) pmpcfg);
            },
            48..=55 => unsafe {
                asm!("csrr {x}, pmpcfg12", x = out(reg) pmpcfg);
            },
            56..=63 => unsafe {
                asm!("csrr {x}, pmpcfg14", x = out(reg) pmpcfg);
            },
            _ => {
                log::info!("Matched None");
                pmpcfg = 0;
            }
        }
        pmpcfg
    }

}

unsafe fn write_mtvec(value: usize) {
    asm!(
        "csrw mtvec, {x}",
        x = in(reg) value
    )
}

fn read_mtvec() -> usize {
    let mtvec: usize;
    unsafe {
        asm!(
            "csrr {x}, mtvec",
            x = out(reg) mtvec
        )
    }
    return mtvec;
}

#[derive(Copy, Clone, Debug)]
pub struct RegisterArguments {
    pub zero: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
}

impl RegisterArguments {
    pub const fn const_default() -> RegisterArguments {
        RegisterArguments {
            zero: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
        }
    }
}

// ————————————————————————— Platform specific code ————————————————————————— //

// M-mode trap handler
// Saves register state - calls trap_handler - restores register state - mret to intended mode.

#[align(4)]
#[unsafe(naked)]
pub extern "C" fn machine_trap_handler() {
    unsafe {
        naked_asm!(
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
        auipc x1, 0x0
        addi x1, x1, 10
        j {trap_handler}
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
        )
    }
}
