//! Performance Counters 
//!
//! This module holds the code used to measure the elapsed time through the different TPM operations.

#[cfg(not(feature = "perf_counters"))]
pub use disabled::{log_boot_measurements, record_counters};
#[cfg(feature = "perf_counters")]
pub use enabled::{log_boot_measurements, record_counters};

pub const BEFORE_TPM2_STARTUP: usize = 0;
pub const AFTER_TPM2_STARTUP: usize = 1;
pub const BEFORE_PCR_EXTEND_1: usize = 2;
pub const AFTER_PCR_EXTEND_1: usize = 3;
pub const BEFORE_PCR_EXTEND_2: usize = 4;
pub const AFTER_PCR_EXTEND_2: usize = 5;
pub const BEFORE_PCR_EXTEND_3: usize = 6;
pub const AFTER_PCR_EXTEND_3: usize = 7;
pub const BEFORE_PCR_EXTEND_4: usize = 8;
pub const AFTER_PCR_EXTEND_4: usize = 9;
pub const BEFORE_PCR_EXTEND_5: usize = 10;
pub const AFTER_PCR_EXTEND_5: usize = 11;
pub const BEFORE_PCR_EXTEND_6: usize = 12;
pub const AFTER_PCR_EXTEND_6: usize = 13;
pub const BEFORE_PCR_EXTEND_7: usize = 14;
pub const AFTER_PCR_EXTEND_7: usize = 15;
pub const BEFORE_DRTM_OPS: usize = 16;
pub const AFTER_DRTM_OPS: usize = 17;
pub const BEFORE_TPM_QUOTE: usize = 18;
pub const AFTER_TPM_QUOTE: usize = 19;

/// No ops, used when the `perf_counters` feature is off.
#[cfg(not(feature = "perf_counters"))]
mod disabled {

    /// Capture the cycle and instruction counters at the given checkpoint.
    pub fn record_counters(_cp: usize) {}

    /// Log the boot time measurements collected by the boot hart.
    pub fn log_boot_measurements(_hartid: usize) {}
}

/// Used when the `perf_counters` feature is on.
#[cfg(feature = "perf_counters")]
mod enabled {
    use crate::arch::{Arch, Architecture};
    use core::sync::atomic::{AtomicU64, Ordering};

    const NB_CHECKPOINTS: usize = 20;

    const CHECKPOINT_LABELS: [&str; NB_CHECKPOINTS] = [
        "before startup",
        "after startup",
        "before pcr_extend 1",
        "after pcr_extend 1",
        "before pcr_extend 2",
        "after pcr_extend 2",
        "before pcr_extend 3",
        "after pcr_extend 3",
        "before pcr_extend 4",
        "after pcr_extend 4",
        "before pcr_extend 5",
        "after pcr_extend 5",
        "before pcr_extend 6",
        "after pcr_extend 6",
        "before pcr_extend 7",
        "after pcr_extend 7",
        "before drtm ops",
        "after drtm ops",
        "before tpm quote",
        "after tpm quote",
    ];

    #[allow(clippy::declare_interior_mutable_const)]
    const ZERO: AtomicU64 = AtomicU64::new(0);
    static MCYCLE: [AtomicU64; NB_CHECKPOINTS] = [ZERO; NB_CHECKPOINTS];
    static MINSTRET: [AtomicU64; NB_CHECKPOINTS] = [ZERO; NB_CHECKPOINTS];

    /// Sample the cycle and instruction counters at the given checkpoint.
    pub fn record_counters(cp: usize) {
        MCYCLE[cp].store(Arch::read_mcycle(), Ordering::SeqCst);
        MINSTRET[cp].store(Arch::read_minstret(), Ordering::SeqCst);
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

