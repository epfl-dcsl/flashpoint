use core::mem::MaybeUninit;
use crate::{platform::exit_failure, tpm_interface::{TPM_RC, quote_verif_info, read_response, tpm20_drtm_operations, tpm20_quote, tpm20_read_pcrs, tpm20_startup, tpm20_pcr_extend}};
use crate::msmt_cfg::*;
use crate::perf_counters::{self, record_counters};

pub unsafe fn scrtm_operations(srtm_measurement_address: usize, srtm_measurement_size: usize) {

    record_counters(perf_counters::BEFORE_TPM2_STARTUP);

    let mut error_code = tpm20_startup();

    record_counters(perf_counters::AFTER_TPM2_STARTUP);

    if error_code != 0 {
        log::error!("Failed to startup the TPM with code {:x}", error_code);
        exit_failure();
    } else {
        log::debug!("Successfully initialized the TPM");
    }
    let pcr_indices = [1, 0, 0];
    let pcr_17_index = [0, 0, 2];
    let mut read_rsp : read_response = MaybeUninit::zeroed().assume_init();

    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest :{:?}", read_rsp.digest);
    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_17_index.as_ptr(), &mut read_rsp);
    log::debug!("\n\nPrinting PCR 17's digest :\n{:?}", read_rsp.digest);
    
    if error_code != 0 {
        log::error!("Failed to read PCRs with code {:x}", error_code);
    } else {
        log::debug!("Successfully read PCRs");
    }

    // PCR 0 extend with PMP STATE 
    record_counters(perf_counters::BEFORE_PCR_EXTEND_1);

    let pcr_extend_resp = tpm20_pcr_extend(0, srtm_measurement_address as *const usize as *const u8, srtm_measurement_size  as u32);

    record_counters(perf_counters::AFTER_PCR_EXTEND_1);

    if pcr_extend_resp != 0 {
        log::error!("Couldn't extend PCR 0 with PMP STATE! Resp: {}", pcr_extend_resp);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with PMP STATE");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with PMP State:{:?} \n", read_rsp.digest);
    
    // PCR 0 extend with Anchor ENTRY_POINT + TEXT  

    record_counters(perf_counters::BEFORE_PCR_EXTEND_2);

    let pcr_extend_resp_1 = tpm20_pcr_extend(0, ANCHOR_ENTRY_TEXT_ADDRESS as *const u8, ANCHOR_ENTRY_TEXT_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_2);
    if pcr_extend_resp_1 != 0 {
        log::error!("Couldn't extend PCR 0 with Anchor ENTRY_POINT + TEXT! Resp: {}", pcr_extend_resp_1);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with Anchor ENTRY_POINT + TEXT");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with Anchor ENTRY_POINT + TEXT:{:?} \n", read_rsp.digest);


    // PCR 0 extend with Anchor RODATA  
    record_counters(perf_counters::BEFORE_PCR_EXTEND_3);
    let pcr_extend_resp_2 = tpm20_pcr_extend(0, ANCHOR_RODATA_ADDRESS as *const u8, ANCHOR_RODATA_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_3);
    if pcr_extend_resp_2 != 0 {
        log::error!("Couldn't extend PCR 0 with Anchor RODATA! Resp: {}", pcr_extend_resp_2);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with Anchor RODATA");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with Anchor RODATA:{:?} \n", read_rsp.digest);


    // PCR 0 extend with Anchor GOT  
    record_counters(perf_counters::BEFORE_PCR_EXTEND_4);
    let pcr_extend_resp_3 = tpm20_pcr_extend(0, ANCHOR_GOT_ADDRESS as *const u8, ANCHOR_GOT_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_4);
    if pcr_extend_resp_3 != 0 {
        log::error!("Couldn't extend PCR 0 with Anchor GOT! Resp: {}", pcr_extend_resp_3);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with Anchor GOT");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with Anchor GOT:{:?} \n", read_rsp.digest);
    
    
    // PCR 0 extend with TPM_DRV ENTRY_POINT + TEXT  
    record_counters(perf_counters::BEFORE_PCR_EXTEND_5);
    let pcr_extend_resp_4 = tpm20_pcr_extend(0, TPM_DRV_ENTRY_TEXT_ADDRESS as *const u8, TPM_DRV_ENTRY_TEXT_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_5);
    if pcr_extend_resp_4 != 0 {
        log::error!("Couldn't extend PCR 0 with TPM_DRV ENTRY_POINT + TEXT! Resp: {}", pcr_extend_resp_4);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with TPM_DRV ENTRY_POINT + TEXT");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with TPM_DRV ENTRY_POINT + TEXT:{:?} \n", read_rsp.digest);


    // PCR 0 extend with Anchor RODATA  
    record_counters(perf_counters::BEFORE_PCR_EXTEND_6);
    let pcr_extend_resp_5 = tpm20_pcr_extend(0, TPM_DRV_RODATA_ADDRESS as *const u8, TPM_DRV_RODATA_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_6);
    if pcr_extend_resp_5 != 0 {
        log::error!("Couldn't extend PCR 0 with TPM_DRV RODATA! Resp: {}", pcr_extend_resp_5);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with TPM_DRV RODATA");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 0's digest after extending with TPM_DRV RODATA:{:?} \n", read_rsp.digest);


    // PCR 0 extend with Anchor GOT  
    record_counters(perf_counters::BEFORE_PCR_EXTEND_7);
    let pcr_extend_resp_6 = tpm20_pcr_extend(0, TPM_DRV_GOT_ADDRESS as *const u8, TPM_DRV_GOT_SIZE  as u32);
    record_counters(perf_counters::AFTER_PCR_EXTEND_7);
    if pcr_extend_resp_6 != 0 {
        log::error!("Couldn't extend PCR 0 with TPM_DRV GOT! Resp: {}", pcr_extend_resp_6);
        exit_failure();
    } else {
        log::debug!("Successfully Extended PCR 0 with TPM_DRV GOT");
    }

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    
    log::info!("Printing PCR 0's digest after SRTM:\n{:?}", read_rsp.digest);
   
    // Printing PCR 17 Digest after SRTM measurements 
    
    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_17_index.as_ptr(), &mut read_rsp);
    log::info!("\n\nPrinting PCR 17's digest after SRTM:\n{:?}", read_rsp.digest);
    
    // Expected digest is per byte array of the following hash - generated using scripts/tpm_pcr_check_0.py 
    // 11e482a46233c30ade30f985c780b9b33b691248817e748fb8ed95595dd1c1cbf1367f1c280eee50afe70b2e4b71086e 
    // log::info!("PCR 0's Expected digest: [17, 228, 130, 164, 98, 51, 195, 10, 222, 48, 249, 133, 199, 128, 185, 179, 59, 105, 18, 72, 129, 126, 116, 143, 184, 237, 149, 89, 93, 209, 193, 203, 241, 54, 127, 28, 40, 14, 238, 80, 175, 231, 11, 46, 75, 113, 8, 110] \n");
    
    // Expected digest is per byte array of the following hash - generated using scripts/tpm_pcr_check_1.py 
    // 72c75778e762ac6a40279302780d747271eefab8ef966652f90926ffdddb834f4bd76e084df29d4a8a4bb37b89195c64 
    // log::info!("PCR 0's Expected digest: [114, 199, 87, 120, 231, 98, 172, 106, 64, 39, 147, 2, 120, 13, 116, 114, 113, 238, 250, 184, 239, 150, 102, 82, 249, 9, 38, 255, 221, 219, 131, 79, 75, 215, 110, 8, 77, 242, 157, 74, 138, 75, 179, 123, 137, 25, 92, 100] \n");

    // let update_count_0 = read_rsp.pcrUpdateCounter;
    // log::info!("PCR Values Digest {:?}, pcr update counter {:?}", read_rsp.pcrValues.digest,  update_count_0);

    if error_code != 0 {
        log::debug!("Failed to read PCRs with code {:x}", error_code);
    } else {
        log::debug!("Successfully read PCRs");
    } 

    //(key_create_resp.handle as u32)
}

//pub unsafe fn dcrtm_operations(exit_region_start: usize, exit_region_size: usize, aikHandle: u32, aikModulus: u8[384]) {
pub unsafe fn dcrtm_operations(drtm_region_start: usize, drtm_region_size: usize) {
    let pcr_indices = [0, 0, 2];          // [1,0,0];
    let mut read_rsp : read_response = MaybeUninit::zeroed().assume_init();
    let mut error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::debug!("Printing PCR 17's digest :{:?}", read_rsp.digest);
    // let update_count_0 = read_rsp.pcrUpdateCounter;
    // log::info!("PCR Values {:?}, pcr update counter {:?}", read_rsp.pcrValues, update_count_0);

    log::info!("Starting DRTM operations with args: 0x{:x}, 0x{:x}.", drtm_region_start, drtm_region_size as u32);
    record_counters(perf_counters::BEFORE_DRTM_OPS);
    let rc = tpm20_drtm_operations(drtm_region_start as *const u8, drtm_region_size as u32);
    record_counters(perf_counters::AFTER_DRTM_OPS);
    log::info!("Done performing DRTM operations with return code: {} .", rc);

    if error_code != 0 {
        log::error!("Failed to read PCRs after DRTM operations with code 0x{:x}", error_code);
    } else {
        log::debug!("Successfully read PCRs after DRTM operations");
    }

    let quote: &mut quote_verif_info = &mut quote_verif_info::new();

    record_counters(perf_counters::BEFORE_TPM_QUOTE);

    let mut quote_error = tpm20_quote(quote);
    if quote_error != 0 {
        log::error!("Failed to create attestation keys or get a quote 0x{:x}.", quote_error);
    } else {
        log::info!("tpm quote response: \n \n ATTESTATION: \n 0x{:?}, \n \n  MODULUS: \n 0x{:?}, \n \n SIGN: \n 0x{:?}", quote.attestation, quote.modulus, quote.signature);
    }
    record_counters(perf_counters::AFTER_TPM_QUOTE);

    read_rsp = MaybeUninit::zeroed().assume_init();
    error_code = tpm20_read_pcrs(pcr_indices.as_ptr(), &mut read_rsp);
    log::info!("Printing PCR 17's digest :{:?}", read_rsp.digest);
    // let update_count_1 = read_rsp.pcrUpdateCounter;
    // log::info!("PCR Values {:?}, pcr update counter {:?}", read_rsp.pcrValues, update_count_1);

    if error_code != 0 {
        log::error!("Failed to read PCRs after TPM Quote with code {:x}", error_code);
    } else {
        log::debug!("Successfully read PCRs after TPM Quote operations");
    }
}