// If we are running the tests or compilling with the `softcore` feature, allow std and main.
// Otherwise, we are in no_std and no_main land.
#![cfg_attr(not(any(test, feature = "softcore")), no_std)]
#![cfg_attr(not(any(test, feature = "softcore")), no_main)]

mod anchor_api;
mod arch;
mod logger;
mod perf_counters;
mod platform;
mod pmp_static;
mod trap;

// Only add the verif module when running tests or verification
#[cfg(any(test, kani, feature = "softcore"))]
mod verif;

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use pmp_static::cfg::{NO_PERMISSIONS, R};
use pmp_static::{print_pmps, range, Range};

use platform::{init, virt::TPM_TIS_BASE_ADDRESS, virt::TPM_TIS_SIZE};

use crate::arch::{pmpaddr_csr_read, pmpcfg_csr_read, RegisterArguments};
use crate::perf_counters::{log_boot_measurements, record_counters, record_counters_tpm_driver};

/// The expected number of harts.
///
/// Configured through the `FLASHPOINT_NUM_HARTS` environment variable.
const NUM_HARTS: usize = match option_env!("FLASHPOINT_NUM_HARTS") {
    Some(value) => match usize::from_str_radix(value, 10) {
        Ok(value) => value,
        Err(_) => panic!("Failed to parse `FLASHPOINT_NUM_HARTS`"),
    },
    None => 2, // Default value
};
const BOOT_HART_ID: usize = 0;

const TPM_DRIVER_ADDR: usize = 0x80080000;
const UNTRUSTED_ADDR: usize = 0x80200000;

const TRUSTED_MEASUREMENT_SIZE: usize = 0x15fac; // text + rodata
const SECURITY_MONITOR_ADDR: usize = 0x80450000;


/// # Safety:
///
/// Each hart will only access the inner array indexed by its hart ID.
/// DO NOT UPDATE the size of the inner arrays here without changing the corresponding assembly in
/// the entry point assembly.
pub static mut RETURN_VALUES: [[u64; 4]; NUM_HARTS] = [[0; _]; _];

/// The stack base address for each of the harts
static mut STACK_ADDRESS: [usize; NUM_HARTS] = {
    /// The start address for the stack segment
    const STACK_BASE_ADDR: usize = 0x80070000;
    /// The size of each of the stacks, overprovisioned, could be smaller
    const STACK_SIZE: usize = 0x4000;

    // Compute the start address of each stack at compile time
    let mut addrs = [0; NUM_HARTS];
    let mut i = 0;
    while i < NUM_HARTS {
        addrs[i] = STACK_BASE_ADDR + (i * STACK_SIZE);
        i += 1;
    }
    addrs
};

#[allow(clippy::declare_interior_mutable_const)]
const ZERO: AtomicU64 = AtomicU64::new(0);
static DOM0_ARG1: AtomicU64 = ZERO;
static DOM0_ARG2: AtomicU64 = ZERO;
static DOM1_ARG1: AtomicU64 = ZERO;
static DOM1_ARG2: AtomicU64 = ZERO;
static DOM1_ENTRY: AtomicU64 = ZERO;
static CORES_DONE_WITH_PMP_INIT: AtomicUsize = AtomicUsize::new(0);
static CORES_RETURNED_FROM_UNTRUSTED: AtomicUsize = AtomicUsize::new(0);

// —————————————————————————— Domains Definitions ——————————————————————————— //
// First, we define the memory regions as `(start, end)`.                     //
// Then, we define all the different M-mode domains.                          //
// Each domain definition specifies which memory regions are accessible to    //
// that domain, along with the corresponding RWX permissions.                 //
// —————————————————————————————————————————————————————————————————————————— //

// Memory region definitions 
/// The anchor itself, this range needs to be excluded from all domains.
const ANCHOR_REGION: (usize, usize) = (0x80000000, 0x80080000);
/// The TPM driver range.
// const TPM_DRIVER_REGION: (usize, usize) = (0x80080000, 0x80100000);
/// Anchor + TPM driver region, to be protected during firmware and SM execution 
const ANCHOR_TPM_DRIVER_REGION: (usize, usize) = (0x80000000, 0x80100000);
/// The TPM device MMIO region
const TPM_DEVICE_REGION: (usize, usize) = (
    TPM_TIS_BASE_ADDRESS,
    TPM_TIS_BASE_ADDRESS + TPM_TIS_SIZE + 0x3000, // TODO: it's rounded up to a power of two for now  
);
/// The untrusted range of memory accessible to the firmware.
const UNTRUSTED_REGION: (usize, usize) = (0x80200000, 0x80400000);
/// The security monitor region, to be used by the TPM driver for measurement 
const SECURITY_MONITOR_REGION: (usize, usize) = (0x80400000, 0x80500000); // bounds adjusted for NAPOT  

// Domain definitions

/// The TPM driver domain.
const TPM_DRIVER_DOMAIN_SRTM: &[Range] = &[
    range(ANCHOR_REGION, R),
    range(UNTRUSTED_REGION, NO_PERMISSIONS),    // Enforcing least privileges, can also remove access to rest of the memory 
];

/// The TPM driver domain.
const TPM_DRIVER_DOMAIN_DRTM: &[Range] = &[
    range(ANCHOR_REGION, NO_PERMISSIONS),
    range(SECURITY_MONITOR_REGION, R),
    range(UNTRUSTED_REGION, NO_PERMISSIONS),    // Enforcing least privileges, can also remove access to rest of the memory 
];

/// The firmware (OpenSBI) domain.
#[cfg(not(feature = "xiangshan"))]
const FIRMWARE_DOMAIN: &[Range] = &[
    range(ANCHOR_TPM_DRIVER_REGION, NO_PERMISSIONS),
    range(TPM_DEVICE_REGION, NO_PERMISSIONS),
];
#[cfg(feature = "xiangshan")]       // There is no TPM device on the Xiangshan platform 
const FIRMWARE_DOMAIN: &[Range] = &[
    range(ANCHOR_TPM_DRIVER_REGION, NO_PERMISSIONS),
];

/// The security monitor itself.
#[cfg(not(feature = "xiangshan"))]
const SECURITY_MONITOR_DOMAIN: &[Range] = &[
    range(ANCHOR_TPM_DRIVER_REGION, NO_PERMISSIONS),
    range(TPM_DEVICE_REGION, NO_PERMISSIONS),
];
#[cfg(feature = "xiangshan")]       // There is no TPM device on the Xiangshan platform 
const SECURITY_MONITOR_DOMAIN: &[Range] = &[
    range(ANCHOR_TPM_DRIVER_REGION, NO_PERMISSIONS),
];

/// The list of domains
const DOMAINS: &[&[Range]] = &[TPM_DRIVER_DOMAIN_SRTM, TPM_DRIVER_DOMAIN_DRTM, FIRMWARE_DOMAIN, SECURITY_MONITOR_DOMAIN];
const TPM_DRIVER_SRTM_DOMAIN_ID: usize = 0;
const TPM_DRIVER_DRTM_DOMAIN_ID: usize = 1;
const FIRMWARE_DOMAIN_ID: usize = 2;
const SECURITY_MONITOR_DOMAIN_ID: usize = 3;

// Finally, we statically compute the PMP tables.
// PMP configurations are computed statically for each domain at compile time 
// and stored in `static` variables. See PMPADDR_TABLE and PMPCFG_TABLE below.

const NB_DOMAINS: usize = DOMAINS.len();
const PMP_ENTRIES: usize = 16;
const PMP_CFG_ENTRIES: usize = PMP_ENTRIES / 8;

const PMP_STATE_WORDS_PER_HART: usize = PMP_CFG_ENTRIES + PMP_ENTRIES;
const PMP_STATE_MEASUREMENT_SIZE: usize =
    PMP_STATE_WORDS_PER_HART * NUM_HARTS * 8;

static PMP_STATE_AFTER_INIT: [AtomicU64; PMP_STATE_WORDS_PER_HART * NUM_HARTS] = [ZERO; PMP_STATE_WORDS_PER_HART * NUM_HARTS]; 

/// The table holding the PMP addresses for each domain.
static PMPADDR_TABLE: [[usize; PMP_ENTRIES]; NB_DOMAINS] = pmp_static::build_pmpaddr_table(DOMAINS);
/// The table holding the PMP configuration for each domain.
static PMPCFG_TABLE: [[usize; PMP_CFG_ENTRIES]; NB_DOMAINS] =
    pmp_static::build_pmpcfg_table(DOMAINS);


// ————————————————————————————— State Machine —————————————————————————————— //
// Each hart keep tracks of its current state. Upon entering the anchor,      //
// the hart looks-up which state it is currently in and calls the             //
// corresponding transition function.                                         //
// —————————————————————————————————————————————————————————————————————————— //

const COLD_BOOT: u64 = 0;
const AFTER_SRTM: u64 = 1;
const AFTER_UNTRUSTED: u64 = 2;
const AFTER_DRTM: u64 = 3;
const AFTER_SM: u64 = 4;    // Should be unreachable 

#[allow(clippy::declare_interior_mutable_const)]
const INITIAL_STATE: AtomicU64 = AtomicU64::new(COLD_BOOT);

/// A table storing the state of each hart.
static HART_STATES: [AtomicU64; NUM_HARTS] = [INITIAL_STATE; NUM_HARTS];

// —————————————————————————————— Entry Point ——————————————————————————————— //

/// The anchor entry point, which dispatch to the appropriate handler based on the current state of
/// the system.
pub extern "C-unwind" fn anchor_main(a0: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> u64 {
    let reg_args = RegisterArguments { a0, a1, a2, a3, a4 };
    let hartid = arch::read_mhartid();
    let state = HART_STATES[hartid].load(Ordering::SeqCst);

    // The main state machine
    let (a0, a1, addr) = match state {
        COLD_BOOT => handle_cold_boot(reg_args, hartid),
        AFTER_SRTM => handle_after_srtm(reg_args, hartid),  // only for boot hart 
        AFTER_UNTRUSTED => handle_after_untrusted(reg_args, hartid),
        AFTER_DRTM => handle_after_drtm(reg_args, hartid),  // only for boot hart 
        AFTER_SM => handle_after_sm(reg_args, hartid),
        _ => {
            panic!("Hart {hartid}: unknown state {state}");
        }
    };

    unsafe {
        let ret_values = &mut RETURN_VALUES[hartid];
        ret_values[0] = a0;
        ret_values[1] = a1;
        ret_values[2] = addr;
    }

    hartid as u64
}

// —————————————————————————— Transition Functions —————————————————————————— //

fn handle_cold_boot(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    init();
    if hartid == BOOT_HART_ID {
        record_counters(perf_counters::COLD_BOOT);
    }
    log::debug!("HART {}: Hello, world! I am THE ANCHOR.", hartid);
    print_pmps();

    if hartid == BOOT_HART_ID {
        handle_cold_boot_boot_hart(reg_args, hartid)
    } else {
        handle_cold_boot_other_hart(reg_args, hartid)
    }
}

fn handle_cold_boot_boot_hart(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    log::debug!("Hart {}: Preparing for TPM driver", hartid);

    write_pmp(TPM_DRIVER_SRTM_DOMAIN_ID);
    print_pmps();

    // Capture PMP State
    save_pmp_after_init(hartid);
    CORES_DONE_WITH_PMP_INIT.fetch_add(1, Ordering::SeqCst);

    record_counters(perf_counters::START_WAIT_FOR_NON_BOOT_INIT);

    // Wait for all harts to save PMP state
    while CORES_DONE_WITH_PMP_INIT.load(Ordering::SeqCst) != NUM_HARTS {
        core::hint::spin_loop();
    }

    record_counters(perf_counters::END_WAIT_FOR_NON_BOOT_INIT);

    DOM0_ARG1.store(reg_args.a0, Ordering::SeqCst);
    DOM0_ARG2.store(reg_args.a1, Ordering::SeqCst);

    // Advance the state machine to the next state 
    HART_STATES[hartid].store(AFTER_SRTM, Ordering::SeqCst);

    record_counters(perf_counters::ENTER_TPM_SRTM);

    // Switch to TPM Driver for the SRTM
    (
        PMP_STATE_AFTER_INIT.as_ptr() as u64,
        PMP_STATE_MEASUREMENT_SIZE as u64,
        TPM_DRIVER_ADDR as u64,
    )
}

fn handle_cold_boot_other_hart(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    log::debug!("Hart {}: Non-boot hart, will go directly to Untrusted", hartid);

    // Configure the PMP for the next transition
    write_pmp(FIRMWARE_DOMAIN_ID);

    print_pmps();

    // Capture PMP State
    save_pmp_after_init(hartid);
    CORES_DONE_WITH_PMP_INIT.fetch_add(1, Ordering::SeqCst);

    // Wait while SRTM is not yet done 
    while HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst) < AFTER_SRTM {
        core::hint::spin_loop();
    }

    // Advance the state machine to the next state 
    HART_STATES[hartid].store(AFTER_UNTRUSTED, Ordering::SeqCst);

    // Jump to Untrusted
    (reg_args.a0, reg_args.a1, UNTRUSTED_ADDR as u64)
}

fn handle_after_srtm(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    record_counters(perf_counters::EXIT_TPM_SRTM);
    record_counters_tpm_driver(perf_counters::TPM_DRV_SRTM, &reg_args);

    log::debug!("Hart {}: Back from TPM Driver after SRTM", hartid);
    assert_eq!(hartid, BOOT_HART_ID);
    write_pmp(FIRMWARE_DOMAIN_ID);
    print_pmps();

    // Advance the state machine to the next state 
    HART_STATES[hartid].store(AFTER_UNTRUSTED, Ordering::SeqCst);

    let a0 = DOM0_ARG1.load(Ordering::SeqCst);
    let a1 = DOM0_ARG2.load(Ordering::SeqCst);

    record_counters(perf_counters::ENTER_UNTRUSTED);

    (a0, a1, UNTRUSTED_ADDR as u64)
}

fn handle_after_untrusted(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    if hartid == BOOT_HART_ID {
        handle_after_untrusted_boot_hart(reg_args, hartid)
    } else {
        handle_after_untrusted_other_hart(reg_args, hartid)
    }
}

fn handle_after_untrusted_boot_hart(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    record_counters(perf_counters::EXIT_UNTRUSTED);

    log::debug!("Hart {}: Back from Untrusted Firmware OpenSBI", hartid);

    let exit_region_entry: u64 = reg_args.a2;
    let exit_region_arg1: u64 = reg_args.a3;
    let exit_region_arg2: u64 = reg_args.a4;

    log::trace!(
        "Hart {}: We just got unlocked! Exit_Region_Entry: 0x{:x} Exit_Region_Arg1: 0x{:x} Exit_Region_Arg2: 0x{:x}",
        hartid,
        exit_region_entry,
        exit_region_arg1,
        exit_region_arg2
    );

    arch::init();
    write_pmp(TPM_DRIVER_DRTM_DOMAIN_ID);
    print_pmps();

    // Advance the state machine to the next state 
    HART_STATES[hartid].store(AFTER_DRTM, Ordering::SeqCst);

    CORES_RETURNED_FROM_UNTRUSTED.fetch_add(1, Ordering::SeqCst);

    DOM1_ENTRY.store(exit_region_entry, Ordering::SeqCst);
    DOM1_ARG1.store(exit_region_arg1, Ordering::SeqCst);
    DOM1_ARG2.store(exit_region_arg2, Ordering::SeqCst);

    record_counters(perf_counters::START_WAIT_FOR_NON_BOOT_UNTRUSTED);

    // Wait until all harts are back from untrusted, then jump to TPM driver 
    while CORES_RETURNED_FROM_UNTRUSTED.load(Ordering::SeqCst) != NUM_HARTS {
        core::hint::spin_loop();
    }

    record_counters(perf_counters::ENTER_TPM_DRTM);

    (
        SECURITY_MONITOR_ADDR as u64,
        TRUSTED_MEASUREMENT_SIZE as u64,
        TPM_DRIVER_ADDR as u64,
    )
}

fn handle_after_untrusted_other_hart(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    let exit_region_entry: u64 = reg_args.a2;
    let exit_region_arg1: u64 = reg_args.a3;
    let exit_region_arg2: u64 = reg_args.a4;

    arch::init(); // Calling arch init to again set up the trap handler and the SP after getting unlocked!

    log::debug!("Hart {}: Back from Untrusted Firmware OpenSBI", hartid);

    log::trace!(
        "Hart {}: We just got unlocked! Exit_Region_Entry: 0x{:x} Exit_Region_Arg1: 0x{:x} Exit_Region_Arg2: 0x{:x}",
        hartid,
        exit_region_entry,
        exit_region_arg1,
        exit_region_arg2
    );

    write_pmp(SECURITY_MONITOR_DOMAIN_ID);
    print_pmps();

    // Advance the state machine to the next state
    HART_STATES[hartid].store(AFTER_SM, Ordering::SeqCst);

    CORES_RETURNED_FROM_UNTRUSTED.fetch_add(1, Ordering::SeqCst);

    // Wait while DRTM is not yet done
    while HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst) < AFTER_SM {
        core::hint::spin_loop();
    }

    arch::reset_core_for_sm();

    (exit_region_arg1, exit_region_arg2, exit_region_entry)
}

fn handle_after_drtm(reg_args: RegisterArguments, hartid: usize) -> (u64, u64, u64) {
    record_counters(perf_counters::EXIT_TPM_DRTM);
    record_counters_tpm_driver(perf_counters::TPM_DRV_DRTM, &reg_args);

    log::debug!("Hart {}: Returned from TPM Driver After DRTM", hartid);

    assert_eq!(hartid, BOOT_HART_ID);
    write_pmp(SECURITY_MONITOR_DOMAIN_ID);
    print_pmps();

    let a0 = DOM1_ARG1.load(Ordering::SeqCst);
    let a1 = DOM1_ARG2.load(Ordering::SeqCst);
    let addr = DOM1_ENTRY.load(Ordering::SeqCst);

    record_counters(perf_counters::ENTER_SM);
    log_boot_measurements(hartid);

    // Advance the state machine to the next state
    HART_STATES[hartid].store(AFTER_SM, Ordering::SeqCst);

    arch::reset_core_for_sm();

    log::trace!("SM Addr: 0x{:x} Arg1: 0x{:x} Arg2: 0x{:x} ", addr, a0, a1);

    (a0, a1, addr)
}

fn handle_after_sm(_reg_args: RegisterArguments, _hartid: usize) -> (u64, u64, u64) {
    panic!("Done with DRTM state should never be reached");
}

/// Configure the PMP for the given domain ID.
fn write_pmp(domain_id: usize) {
    let pmpaddr = PMPADDR_TABLE[domain_id];
    let pmpcfg = PMPCFG_TABLE[domain_id];

    for (idx, addr) in pmpaddr.iter().enumerate() {
        arch::pmpaddr_csr_write(idx, *addr);
    }
    for (idx, cfg) in pmpcfg.iter().enumerate() {
        arch::pmpcfg_csr_write(idx * 8, *cfg);
    }
    arch::sfence_vma();
}

fn save_pmp_after_init(hartid: usize) {
    const HART_OFFSET: usize = PMP_CFG_ENTRIES + PMP_ENTRIES;

    // Capture PMP State - Warning: This assumes BOOT_HART is 0!!!
    for id in 0..PMP_CFG_ENTRIES {
        // log::info!(
        //     "Hart {}: Setting PMP_STATE[{}] to {:x}",
        //     hartid,
        //     hartid * HART_OFFSET + id,
        //     pmpcfg_csr_read(id * 8)
        // );
        PMP_STATE_AFTER_INIT[hartid * HART_OFFSET + id]
            .store(pmpcfg_csr_read(id * 8), Ordering::SeqCst);
    }
    for id in 0..PMP_ENTRIES {
        // log::info!(
        //     "Hart {}: Setting PMP_STATE[{}] to {:x}",
        //     hartid,
        //     hartid * HART_OFFSET + id + PMP_CFG_ENTRIES,
        //     pmpaddr_csr_read(id)
        // );
        PMP_STATE_AFTER_INIT[hartid * HART_OFFSET + id + PMP_CFG_ENTRIES]
            .store(pmpaddr_csr_read(id), Ordering::SeqCst);
    }
}

#[panic_handler]
#[cfg(not(any(test, kani, feature = "softcore")))]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("Panicked at {:#?} ", info);
    platform::exit_failure();
}

/// The entry point when running in user-space
#[cfg(any(test, feature = "softcore"))]
fn main() {
    arch::_start();
}
