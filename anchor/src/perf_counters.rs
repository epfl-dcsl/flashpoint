//! Performance Counters Measurements
//!
//! This module holds the code used for measuring the elapsed time through the different anchor
//! transitions.

#[cfg(not(feature = "perf_counters"))]
pub use disabled::{log_boot_measurements, record_counters, record_counters_tpm_driver};
#[cfg(feature = "perf_counters")]
pub use enabled::{log_boot_measurements, record_counters, record_counters_tpm_driver};

/// After the platform initialization, on cold boot.
pub const COLD_BOOT: usize = 0;
/// Boot core starting the wait for non boot cores' platform init 
pub const START_WAIT_FOR_NON_BOOT_INIT: usize = 1;
/// Boot core finishing the wait for non boot cores' platform init 
pub const END_WAIT_FOR_NON_BOOT_INIT: usize = 2;
/// Before jumping into the TPM driver for the SRTM.
pub const ENTER_TPM_SRTM: usize = 3;
/// The TPM driver's own counters, captured at SRTM entry point.
pub const TPM_DRV_SRTM: usize = 4;
/// Back from the TPM driver, after the SRTM.
pub const EXIT_TPM_SRTM: usize = 5;
/// Before jumping into the untrusted firmware.
pub const ENTER_UNTRUSTED: usize = 6;
/// Back from the untrusted firmware.
pub const EXIT_UNTRUSTED: usize = 7;
/// Boot core starting the wait for non boot cores' untrusted execution  
pub const START_WAIT_FOR_NON_BOOT_UNTRUSTED: usize = 8;
 /// Before jumping into the TPM driver for the DRTM.
pub const ENTER_TPM_DRTM: usize = 9;
/// The TPM driver's own counters, captured at DRTM entry point.
pub const TPM_DRV_DRTM: usize = 10;
/// Back from the TPM driver, after the DRTM.
pub const EXIT_TPM_DRTM: usize = 11;
/// Before transferring control to the security monitor.
pub const ENTER_SM: usize = 12;

/// No ops, used when the `perf_counters` feature is off.
#[cfg(not(feature = "perf_counters"))]
mod disabled {
    use crate::arch::RegisterArguments;

    /// Sample the cycle and instruction counters at the given checkpoint.
    pub fn record_counters(_cp: usize) {}

    /// Record the counters measured by the TPM driver, which reports them in a2 and a3.
    pub fn record_counters_tpm_driver(_cp: usize, _reg_args: &RegisterArguments) {}

    /// Log the boot time measurements collected by the boot hart.
    pub fn log_boot_measurements(_hartid: usize) {}
}

/// Used when the `perf_counters` feature is on.
#[cfg(feature = "perf_counters")]
mod enabled {
    use crate::arch::{read_mcycle, read_minstret, RegisterArguments};
    use core::sync::atomic::{AtomicU64, Ordering};

    const NB_CHECKPOINTS: usize = 13;

    const CHECKPOINT_LABELS: [&str; NB_CHECKPOINTS] = [
        "after init",
        "start non-boot init wait",
        "end non-boot init wait",
        "before srtm",
        "after tpm_drv_entry_srtm",
        "after srtm",
        "before untrusted",
        "after untrusted",
        "start non-boot untrusted wait",
        "before drtm",
        "after tpm_drv_entry_drtm",
        "after drtm",
        "before sm",
    ];

    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    static MCYCLE: [AtomicU64; NB_CHECKPOINTS] = [ZERO; NB_CHECKPOINTS];
    static MINSTRET: [AtomicU64; NB_CHECKPOINTS] = [ZERO; NB_CHECKPOINTS];

    /// Sample the cycle and instruction counters at the given checkpoint.
    pub fn record_counters(cp: usize) {
        MCYCLE[cp].store(read_mcycle(), Ordering::SeqCst);
        MINSTRET[cp].store(read_minstret(), Ordering::SeqCst);
    }

    /// Record the counters measured by the TPM driver, which reports them in a2 and a3.
    pub fn record_counters_tpm_driver(cp: usize, reg_args: &RegisterArguments) {
        MCYCLE[cp].store(reg_args.a3, Ordering::SeqCst);
        MINSTRET[cp].store(reg_args.a2, Ordering::SeqCst);
    }

    /// Log the boot time measurements collected by the boot hart.
    pub fn log_boot_measurements(hartid: usize) {
        log::info!("Boot time measurements for hart {}:", hartid);
        for cp in 0..NB_CHECKPOINTS {
            log::info!(
                "  {:<20} mcycle: {:>12}  minstret: {:>12}",
                CHECKPOINT_LABELS[cp],
                MCYCLE[cp].load(Ordering::SeqCst),
                MINSTRET[cp].load(Ordering::SeqCst)
            );
        }
    }
}
