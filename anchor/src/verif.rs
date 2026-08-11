//! Fuzzing and Model Checking

#![cfg_attr(not(test), allow(unused))]

use super::arch::*;
use crate::arch::csr::*;
use crate::pmp_static::cfg::{L, R, W, X};
use crate::{
    write_pmp, DOMAINS, FIRMWARE_DOMAIN_ID, PMPADDR_TABLE, PMPCFG_TABLE, PMP_CFG_ENTRIES,
    PMP_ENTRIES, TPM_DRIVER_DOMAIN_ID,
};
use crate::{
    BOOT_HART_ID, COLD_BOOT, CORES_DONE_WITH_PMP_INIT, CORES_RETURNED_FROM_UNTRUSTED,
    AFTER_SM, AFTER_UNTRUSTED, AFTER_SRTM, AFTER_DRTM, HART_STATES,
    SECURITY_MONITOR_DOMAIN_ID,
};
use core::sync::atomic::Ordering;
use softcore_asm_rv64::softcore_rv64::prelude::*;
use softcore_asm_rv64::softcore_rv64::registers::X0;
use softcore_asm_rv64::softcore_rv64::{config, new_core, raw};

const OTHER_HART_ID: usize = 1;

fn verify_pmp(domain_id: usize, addr: u64) {
    let spec = DOMAINS[domain_id];
    let pmp_addr = PMPADDR_TABLE[domain_id];
    let pmp_cfg = PMPCFG_TABLE[domain_id];
    let read = raw::AccessType::Read(());
    let write = raw::AccessType::Write(());
    let fetch = raw::AccessType::InstructionFetch(());

    SOFT_CORE.with_borrow_mut(|core| {
        // Check that there is one PMP locked per region
        for i in 0..spec.len() {
            // we avoid the modulo to help the model checker here
            // cfg_shoft = (i % 4) * 8
            let cfg_shif = (i & 0b111) << 3;
            let cfg_idx = i >> 3;
            assert!(core.pmpcfg_n[i].bits.bits() as u8 & L != 0);
            assert_eq!(
                core.pmpcfg_n[i].bits.bits() as u8 & !L,
                (pmp_cfg[cfg_idx] >> cfg_shif) as u8 & !L,
            );
            assert_eq!(core.pmpaddr_n[i].bits(), pmp_addr[i] as u64);
        }

        // Check the access rights for each region
        for region in spec {
            let start = region.start as u64;
            let end = region.end as u64;
            let perm = region.perm;

            if start <= addr && addr < end {
                if perm & R == 0 {
                    assert!(core.pmp_check(addr, read).is_some());
                } else {
                    assert!(core.pmp_check(addr, read).is_none());
                }
                if perm & W == 0 {
                    assert!(core.pmp_check(addr, write).is_some());
                } else {
                    assert!(core.pmp_check(addr, write).is_none());
                }
                if perm & X == 0 {
                    assert!(core.pmp_check(addr, fetch).is_some());
                } else {
                    assert!(core.pmp_check(addr, fetch).is_none());
                }
            }
        }
    });
}

fn prepare_pmp(domain_id: usize) {
    write_pmp(domain_id);
    SOFT_CORE.with_borrow_mut(|core| core.execute(raw::ast::EXIT_ANCHOR((X0, X0))));
}

fn prepare_anchor_coold_boot_boot_hart() {
    reset_core(BOOT_HART_ID);
    HART_STATES[BOOT_HART_ID].store(COLD_BOOT, Ordering::SeqCst);
    CORES_DONE_WITH_PMP_INIT.store(1, Ordering::SeqCst);
    SOFT_CORE.with_borrow_mut(|core| {
        core.csrrwi(X0, MIE, 0).expect("Failed to initialize MIE");
    })
}

fn prepare_anchor_coold_boot_other_hart() {
    reset_core(OTHER_HART_ID);
    HART_STATES[OTHER_HART_ID].store(COLD_BOOT, Ordering::SeqCst);
    CORES_DONE_WITH_PMP_INIT.store(1, Ordering::SeqCst);
    HART_STATES[BOOT_HART_ID].store(AFTER_UNTRUSTED, Ordering::SeqCst);
    SOFT_CORE.with_borrow_mut(|core| {
        core.csrrwi(X0, MIE, 0).expect("Failed to initialize MIE");
    })
}

fn prepare_anchor_tpm1_done() {
    reset_core(BOOT_HART_ID);
    HART_STATES[BOOT_HART_ID].store(AFTER_SRTM, Ordering::SeqCst);
    prepare_pmp(TPM_DRIVER_DOMAIN_ID);
}

fn prepare_anchor_srtm_done_boot_hart() {
    reset_core(BOOT_HART_ID);
    HART_STATES[BOOT_HART_ID].store(AFTER_UNTRUSTED, Ordering::SeqCst);
    CORES_RETURNED_FROM_UNTRUSTED.store(1, Ordering::SeqCst);
    prepare_pmp(FIRMWARE_DOMAIN_ID);
}

fn prepare_anchor_srtm_done_other_hart() {
    reset_core(OTHER_HART_ID);
    HART_STATES[OTHER_HART_ID].store(AFTER_UNTRUSTED, Ordering::SeqCst);
    HART_STATES[BOOT_HART_ID].store(AFTER_SM, Ordering::SeqCst);
    prepare_pmp(FIRMWARE_DOMAIN_ID);
}

fn prepare_anchor_tpm2_done() {
    reset_core(BOOT_HART_ID);
    HART_STATES[BOOT_HART_ID].store(AFTER_DRTM, Ordering::SeqCst);
    prepare_pmp(TPM_DRIVER_DOMAIN_ID);
}

fn reset_core(hartid: usize) {
    SOFT_CORE.with_borrow_mut(|core| {
        *core = new_core(config::U74);
        core.reset();
        core.mhartid = bv(hartid as u64);
    });
}

fn havok_random(random: &[u64; 67]) {
    SOFT_CORE.with_borrow_mut(|core| {
        // M-mode CSRs
        core.set_csr(MIE, random[0]);
        core.set_csr(MIP, random[1]);
        core.set_csr(MEPC, random[2]);
        core.set_csr(MTVEC, random[3]);
        core.set_csr(MTVAL, random[4]);
        core.set_csr(MCAUSE, random[5]);
        core.set_csr(MSTATUS, random[6]);
        core.set_csr(MENVCFG, random[7]);
        core.set_csr(MEDELEG, random[8]);
        core.set_csr(MIDELEG, random[9]);
        core.set_csr(MSCRATCH, random[10]);

        // S-mode CSRs
        core.set_csr(SIE, random[10]);
        core.set_csr(STVEC, random[11]);
        core.set_csr(SENVCFG, random[12]);
        core.set_csr(SSCRATCH, random[13]);
        core.set_csr(SEPC, random[13]);
        core.set_csr(SCAUSE, random[14]);
        core.set_csr(STVAL, random[15]);
        core.set_csr(SATP, random[17]);

        let offset = 17;
        for i in 0..PMP_CFG_ENTRIES {
            core.set_csr(PMPCFG0 + i as u64, random[offset + i]);
        }
        let offset = offset + PMP_CFG_ENTRIES;
        for i in 0..PMP_ENTRIES {
            core.set_csr(PMPADDR0 + i as u64, random[offset + i]);
        }

        // General purpose registers
        let offset = offset + PMP_CFG_ENTRIES;
        core.x1 = bv(random[offset + 1]);
        core.x2 = bv(random[offset + 2]);
        core.x3 = bv(random[offset + 3]);
        core.x4 = bv(random[offset + 4]);
        core.x5 = bv(random[offset + 5]);
        core.x6 = bv(random[offset + 6]);
        core.x7 = bv(random[offset + 7]);
        core.x8 = bv(random[offset + 8]);
        core.x9 = bv(random[offset + 9]);
        core.x10 = bv(random[offset + 10]);
        core.x11 = bv(random[offset + 11]);
        core.x12 = bv(random[offset + 12]);
        core.x13 = bv(random[offset + 13]);
        core.x14 = bv(random[offset + 14]);
        core.x15 = bv(random[offset + 15]);
        core.x16 = bv(random[offset + 16]);
        core.x17 = bv(random[offset + 17]);
        core.x18 = bv(random[offset + 18]);
        core.x19 = bv(random[offset + 19]);
        core.x20 = bv(random[offset + 20]);
        core.x21 = bv(random[offset + 21]);
        core.x22 = bv(random[offset + 22]);
        core.x23 = bv(random[offset + 23]);
        core.x24 = bv(random[offset + 24]);
        core.x25 = bv(random[offset + 25]);
        core.x26 = bv(random[offset + 26]);
        core.x27 = bv(random[offset + 27]);
        core.x28 = bv(random[offset + 28]);
        core.x29 = bv(random[offset + 29]);
        core.x30 = bv(random[offset + 30]);
        core.x31 = bv(random[offset + 31]);
    });
}

#[cfg(kani)]
fn havoc_symbolic() {
    SOFT_CORE.with_borrow_mut(|core| {
        // M-mode CSRs
        core.set_csr(MIE, kani::any());
        core.set_csr(MIP, kani::any());
        core.set_csr(MEPC, kani::any());
        core.set_csr(MTVEC, kani::any());
        core.set_csr(MTVAL, kani::any());
        core.set_csr(MCAUSE, kani::any());
        core.set_csr(MSTATUS, kani::any());
        core.set_csr(MENVCFG, kani::any());
        core.set_csr(MEDELEG, kani::any());
        core.set_csr(MIDELEG, kani::any());
        core.set_csr(MSCRATCH, kani::any());

        // S-mode CSRs
        core.set_csr(SIE, kani::any());
        core.set_csr(STVEC, kani::any());
        core.set_csr(SENVCFG, kani::any());
        core.set_csr(SSCRATCH, kani::any());
        core.set_csr(SEPC, kani::any());
        core.set_csr(SCAUSE, kani::any());
        core.set_csr(STVAL, kani::any());
        core.set_csr(SATP, kani::any());

        for i in 0..PMP_CFG_ENTRIES {
            core.set_csr(PMPCFG0 + i as u64, kani::any());
        }
        for i in 0..PMP_ENTRIES {
            core.set_csr(PMPADDR0 + i as u64, kani::any());
        }

        // General purpose registers
        core.x1 = bv(kani::any());
        core.x2 = bv(kani::any());
        core.x3 = bv(kani::any());
        core.x4 = bv(kani::any());
        core.x5 = bv(kani::any());
        core.x6 = bv(kani::any());
        core.x7 = bv(kani::any());
        core.x8 = bv(kani::any());
        core.x9 = bv(kani::any());
        core.x10 = bv(kani::any());
        core.x11 = bv(kani::any());
        core.x12 = bv(kani::any());
        core.x13 = bv(kani::any());
        core.x14 = bv(kani::any());
        core.x15 = bv(kani::any());
        core.x16 = bv(kani::any());
        core.x17 = bv(kani::any());
        core.x18 = bv(kani::any());
        core.x19 = bv(kani::any());
        core.x20 = bv(kani::any());
        core.x21 = bv(kani::any());
        core.x22 = bv(kani::any());
        core.x23 = bv(kani::any());
        core.x24 = bv(kani::any());
        core.x25 = bv(kani::any());
        core.x26 = bv(kani::any());
        core.x27 = bv(kani::any());
        core.x28 = bv(kani::any());
        core.x29 = bv(kani::any());
        core.x30 = bv(kani::any());
        core.x31 = bv(kani::any());
    });
}

fn enter_anchor() {
    use ::softcore_asm_rv64::softcore_rv64::{ast, registers as reg};

    SOFT_CORE.with_borrow_mut(|core| {
        match core.execute(ast::ENTER_ANCHOR((reg::X0, reg::X0))) {
            raw::ExecutionResult::Retire_Success(_) => (),
            _ => panic!("enter_anchor raised a trap"),
        };
    });
}

fn sanitize_address(addr: u64) -> u64 {
    // Align address to 8 bytes, with max 56 bits of address space
    addr & 0x00FFFFFFFFFFFFF8
}

fn check_sm_state_is_clean() {
    SOFT_CORE.with_borrow_mut(|core| {
        assert_eq!(core.mstatus.bits.bits(), 0xa00000000);
        assert_eq!(core.mideleg.bits.bits(), 0);
        assert_eq!(core.medeleg.bits.bits(), 0);
        assert_eq!(core.menvcfg.bits.bits(), 0);
        assert_eq!(core.mie.bits.bits(), 0);
        assert_eq!(core.mip.bits.bits(), 0);
        assert_eq!(core.mtvec.bits.bits(), 0);
        assert_eq!(core.mcause.bits.bits(), 0);
        assert_eq!(core.mtval.bits(), 0);
        assert_eq!(core.mtvec.bits.bits(), 0);
        assert_eq!(core.mepc.bits(), 0);
        assert_eq!(core.mscratch.bits(), 0);
        assert_eq!(core.x1.bits(), 0);
        assert_eq!(core.x2.bits(), 0);
        assert_eq!(core.x3.bits(), 0);
        assert_eq!(core.x4.bits(), 0);
        // Skip x5 (t0)
        assert_eq!(core.x6.bits(), 0);
        assert_eq!(core.x7.bits(), 0);
        assert_eq!(core.x8.bits(), 0);
        assert_eq!(core.x9.bits(), 0);
        // Skip x10 and x11 (a0 and a1)
        assert_eq!(core.x12.bits(), 0);
        assert_eq!(core.x13.bits(), 0);
        assert_eq!(core.x14.bits(), 0);
        assert_eq!(core.x15.bits(), 0);
        assert_eq!(core.x16.bits(), 0);
        assert_eq!(core.x17.bits(), 0);
        assert_eq!(core.x18.bits(), 0);
        assert_eq!(core.x19.bits(), 0);
        assert_eq!(core.x20.bits(), 0);
        assert_eq!(core.x21.bits(), 0);
        assert_eq!(core.x22.bits(), 0);
        assert_eq!(core.x23.bits(), 0);
        assert_eq!(core.x24.bits(), 0);
        assert_eq!(core.x25.bits(), 0);
        assert_eq!(core.x26.bits(), 0);
        assert_eq!(core.x27.bits(), 0);
        assert_eq!(core.x28.bits(), 0);
        assert_eq!(core.x29.bits(), 0);
        assert_eq!(core.x30.bits(), 0);
        assert_eq!(core.x31.bits(), 0);
    });
}

// ————————————————————————————————— Tests —————————————————————————————————— //

#[test]
fn read_write_csr() {
    // Configure the RISC-V core here
    SOFT_CORE.with_borrow_mut(|core| {
        core.mhartid = bv(42);
    });

    // Then try to read a few CSRs
    // Note: if not configured explicitely, most default values are 0
    assert_eq!(read_mhartid(), 42);
    assert_eq!(read_mepc(), 0);
    unsafe { write_mepc(0xff00) };
    assert_eq!(read_mepc(), 0xff00);
}

#[test]
fn test_cold_boot_boot() {
    reset_core(BOOT_HART_ID);
    prepare_anchor_coold_boot_boot_hart();

    _start();

    verify_pmp(TPM_DRIVER_DOMAIN_ID, sanitize_address(0x8000000));
}

// ———————————————————————————————— Fuzzing ————————————————————————————————— //

#[test]
fn fuzz_cold_boot_boot() {
    bolero::check!().with_type().cloned().for_each(|addr: u64| {
        prepare_anchor_coold_boot_boot_hart();

        _start();

        verify_pmp(TPM_DRIVER_DOMAIN_ID, sanitize_address(addr));
        assert_eq!(
            HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
            AFTER_SRTM
        );
    })
}

#[test]
fn fuzz_cold_boot_other() {
    bolero::check!().with_type().cloned().for_each(|addr: u64| {
        prepare_anchor_coold_boot_other_hart();

        _start();

        verify_pmp(FIRMWARE_DOMAIN_ID, sanitize_address(addr));
        assert_eq!(
            HART_STATES[OTHER_HART_ID].load(Ordering::SeqCst),
            AFTER_UNTRUSTED
        );
    })
}

#[test]
fn fuzz_tpm1_done() {
    bolero::check!()
        .with_type()
        .cloned()
        .for_each(|(addr, registers): (u64, [u64; 67])| {
            prepare_anchor_tpm1_done();
            havok_random(&registers);

            enter_anchor();
            _start();

            verify_pmp(FIRMWARE_DOMAIN_ID, sanitize_address(addr));
            assert_eq!(
                HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
                AFTER_UNTRUSTED
            );
        })
}

#[test]
fn fuzz_srtm_done_boot() {
    bolero::check!()
        .with_type()
        .cloned()
        .for_each(|(addr, registers): (u64, [u64; 67])| {
            prepare_anchor_srtm_done_boot_hart();
            havok_random(&registers);

            enter_anchor();
            _start();

            verify_pmp(TPM_DRIVER_DOMAIN_ID, sanitize_address(addr));
            assert_eq!(
                HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
                AFTER_DRTM
            );
        })
}

#[test]
fn fuzz_srtm_done_other() {
    bolero::check!()
        .with_type()
        .cloned()
        .for_each(|(addr, registers): (u64, [u64; 67])| {
            prepare_anchor_srtm_done_other_hart();
            havok_random(&registers);

            enter_anchor();
            _start();

            verify_pmp(SECURITY_MONITOR_DOMAIN_ID, sanitize_address(addr));
            assert_eq!(
                HART_STATES[OTHER_HART_ID].load(Ordering::SeqCst),
                AFTER_SM
            );
        })
}

#[test]
fn fuzz_tpm2_done() {
    bolero::check!()
        .with_type()
        .cloned()
        .for_each(|(addr, registers): (u64, [u64; 67])| {
            prepare_anchor_tpm2_done();
            havok_random(&registers);

            enter_anchor();
            _start();

            verify_pmp(SECURITY_MONITOR_DOMAIN_ID, sanitize_address(addr));
            assert_eq!(
                HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
                AFTER_SM
            );
        });
    check_sm_state_is_clean();
}

// ————————————————————————————— Model Checking ————————————————————————————— //

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_cold_boot_boot() {
    prepare_anchor_coold_boot_boot_hart();
    let addr = kani::any();

    _start();

    verify_pmp(TPM_DRIVER_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
        AFTER_SRTM
    );
}

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_cold_boot_other() {
    prepare_anchor_coold_boot_other_hart();
    let addr = kani::any();

    _start();

    verify_pmp(FIRMWARE_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[OTHER_HART_ID].load(Ordering::SeqCst),
        AFTER_UNTRUSTED
    );
}

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_tpm1_done() {
    prepare_anchor_tpm1_done();
    havoc_symbolic();
    let addr = kani::any();

    enter_anchor();
    _start();

    verify_pmp(FIRMWARE_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
        AFTER_UNTRUSTED
    );
}

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_srtm_done_boot() {
    prepare_anchor_srtm_done_boot_hart();
    havoc_symbolic();
    let addr = kani::any();

    enter_anchor();
    _start();

    verify_pmp(TPM_DRIVER_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
        AFTER_DRTM
    );
}

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_srtm_done_other() {
    prepare_anchor_srtm_done_other_hart();
    havoc_symbolic();
    let addr = kani::any();

    enter_anchor();
    _start();

    verify_pmp(SECURITY_MONITOR_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[OTHER_HART_ID].load(Ordering::SeqCst),
        AFTER_SM
    );
}

#[cfg(kani)]
#[kani::proof]
#[allow(unused)]
fn kani_tpm2_done() {
    prepare_anchor_tpm2_done();
    havoc_symbolic();
    let addr = kani::any();

    enter_anchor();
    _start();

    verify_pmp(SECURITY_MONITOR_DOMAIN_ID, sanitize_address(addr));
    assert_eq!(
        HART_STATES[BOOT_HART_ID].load(Ordering::SeqCst),
        AFTER_SM
    );
    check_sm_state_is_clean();
}
