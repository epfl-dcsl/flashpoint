#![no_std]
#![no_main]
#![feature(fn_align)]
#![feature(naked_functions)]

mod perf_counters;
mod arch;
mod logger;
mod platform;
mod trap;
mod pmp;
mod tpm_interface;
mod crtm;
mod msmt_cfg;

use arch::{Arch, Architecture, FIRMWARE_STACK_POINTER};
use spin::Mutex;
use core::sync::atomic::{AtomicI64, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use core::{arch::asm, sync::atomic::AtomicBool};
use core::panic::PanicInfo;
use pmp::print_pmps;

use platform::{exit_failure, exit_success, init};

use crate::arch::RegisterArguments;
use crate::crtm::{dcrtm_operations, scrtm_operations};
use crate::perf_counters::log_boot_measurements;

static COLD_BOOT: AtomicBool = AtomicBool::new(true);
const PMP_ENTRIES: usize = 16;

static AIK_HANDLE: AtomicU32 = AtomicU32::new(0);
const U8_ZERO: AtomicU8 = AtomicU8::new(0);
static AIK_MODULUS: [AtomicU8; 364] = [U8_ZERO; 364];

#[no_mangle]
#[link_section = ".entry_point"]
pub unsafe extern "C" fn _start() -> ! {
    // Address of the top of the stack (stack grow towards lower addresses)
    // static STACK: &'static usize = unsafe { &_stack_end };
    // Initialize stack pointer and jump into main
    // TODO: zero-out the BSS (QEMU might do it for us, but real hardware will not)
    asm!(
        //"li sp, 0x800f0000",   // Hard-coding ToS.... :| 
        //"csrr t0, mhartid",
        "csrr t0, minstret",
        "csrr t2, mcycle",
        //"slli t0, t0, 3",   // to index into STACK_ADDRESS
        "la t1, {stack}",
        //"add t1, t1, t0",
        "ld t1, 0(t1)",
        "mv sp, t1",
        "addi sp, sp, -9*8",
        "sd zero, 0*8(sp)",
        "sd a0, 1*8(sp)",
        "sd a1, 2*8(sp)",
        "sd a2, 3*8(sp)",
        "sd a3, 4*8(sp)",
        "sd a4, 5*8(sp)",
        "sd a5, 6*8(sp)",
        "sd a6, 7*8(sp)",
        "sd a7, 8*8(sp)",
        "mv a0, sp",
        "mv a1, t0",
        "mv a2, t2",
        "j {main}",
        //in("t0") STACK_ADDRESS[hartid],
        stack = sym FIRMWARE_STACK_POINTER,
        main = sym main,
        //clobber_abi("C"),
        options(noreturn)
    );
}

#[cfg(not(feature = "xiangshan"))]
extern "C" fn main(reg_args: &mut RegisterArguments, entry_minstret: usize, entry_mcycle: usize) -> ! {
    if COLD_BOOT.load(Ordering::SeqCst) {
        init();

        log::info!("\n\nHello, world! I am THE TPM Driver.\n\n");
        COLD_BOOT.store(false, Ordering::SeqCst);

        // Print the state of the PMPs from within the TPM driver. 
        print_pmps();

        unsafe {
            scrtm_operations(reg_args.a0, reg_args.a1);    //TODO - just use 0x80000000
        }

        log::info!("I am gonna go back to the anchor now!");

        // TPM Driver doesn't know where the anchor should go next ... So it will say "IDK" i.e. 0x49444B
        unsafe { Arch::enter_anchor(0x49444B, 0x49444B, entry_minstret, entry_mcycle); }

    } else {
        let drtm_region_start: usize = (reg_args.a0 << 32) >> 32; // TODO: IDK why this is needed :|
        let drtm_region_size: usize = reg_args.a1;
        let tpm_quote_resp_addr: usize = reg_args.a2;

        Arch::init();   // Calling arch init to again set up the trap handler and the SP after getting unlocked! 
        log::info!("\n\nBack in TPM Driver! DRTM_Region_Start_Addr: 0x{:x} DRTM_Region_Size: 0x{:x} TPM_Quote_Resp_Addr: 0x{:x}\n\n", drtm_region_start, drtm_region_size, tpm_quote_resp_addr);


        // Print the state of the PMPs from within the TPM driver. 
        print_pmps();

        //TPM operations D-CRTM 
        unsafe { dcrtm_operations(drtm_region_start, drtm_region_size); }
        
        log_boot_measurements(0);   // expecting boot hart id

        log::info!("I am gonna go back to the anchor now!");

        // TPM Driver doesn't know where the anchor should go next ... So it will say "IDK" i.e. 0x49444B
        unsafe { Arch::enter_anchor(0x49444B, 0x49444B, entry_minstret, entry_mcycle); }
    }
}

#[cfg(feature = "xiangshan")]
extern "C" fn main(reg_args: &mut RegisterArguments, entry_minstret: usize, entry_mcycle: usize) -> ! {
    if COLD_BOOT.load(Ordering::SeqCst) {
        init();
        COLD_BOOT.store(false, Ordering::SeqCst);
        // TPM Driver doesn't know where the anchor should go next ... So it will say "IDK" i.e. 0x49444B
        unsafe { Arch::enter_anchor(0x49444B, 0x49444B, entry_minstret, entry_mcycle); }

    } else {
        Arch::init();   // Calling arch init to again set up the trap handler and the SP after getting unlocked! 
        // TPM Driver doesn't know where the anchor should go next ... So it will say "IDK" i.e. 0x49444B
        unsafe { Arch::enter_anchor(0x49444B, 0x49444B, entry_minstret, entry_mcycle); }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("Panicked at {:#?} ", info);
    exit_failure();
}
