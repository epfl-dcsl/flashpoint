// Implementation of a TPM driver for the TPM TIS interface
//
// Copyright (C) 2006-2011 IBM Corporation
//
// Authors:
//     Stefan Berger <stefanb@linux.vnet.ibm.com>
//
// This file may be distributed under the terms of the GNU LGPLv3 license.

/*#include "byteorder.h" // be32_to_cpu*/
#include "../tpm_includes/tpm_driver.h"// struct tpm_driver
#include <stdbool.h>
#include <stdint.h>
#include "../tpm_includes/swap.h"
#include "../tpm_includes/tcg.h"
#include "../tpm_includes/riscv_io.h"

void *memcpy(void *dest, const void *src, u64 n)
{
    u64  i;

    for ( u64 i = 0; i < n; ++i){
        ((unsigned char *) dest)[i] = ((unsigned char *) src)[i];
    }
    return dest;
}

struct predicate_args{
	u8* reg;
	u8 mask;
	u8 expect;
};

static bool reg_predicate(void * args){
	struct predicate_args arg = *((struct predicate_args* ) args);
	u8 val = readl(arg.reg);
	return (val & arg.mask) == arg.expect;
}

#define NULL 0x0

#define write_l(c, v) writel(v, c)
#define write_b(c, v) writeb(v, c)


#define timer_check(end) sbi_timer_get_device()->timer_value - end > 0

/* low level driver implementation */
struct tpm_driver {
    u32 *timeouts;
    u32 *durations;
    void (*set_timeouts)(u32 timeouts[4], u32 durations[3]);
    u32 (*probe)(void);
    TPMVersion (*get_tpm_version)(void);
    u32 (*init)(void);
    u32 (*activate)(u8 locty);
    u32 (*ready)(void);
    u32 (*senddata)(const u8 *const data, u32 len);
    u32 (*readresp)(u8 *buffer, u32 *len);
    u32 (*waitdatavalid)(void);
    u32 (*waitrespready)(enum tpmDurationType to_t);
};


#define TIS_DRIVER_IDX       0
#define CRB_DRIVER_IDX       1
#define TPM_NUM_DRIVERS      2

extern struct tpm_driver tpm_drivers[TPM_NUM_DRIVERS];
#define TPM_INVALID_DRIVER   0xf

static const u32 tis_default_timeouts[4] = {
    TIS_DEFAULT_TIMEOUT_A,
    TIS_DEFAULT_TIMEOUT_B,
    TIS_DEFAULT_TIMEOUT_C,
    TIS_DEFAULT_TIMEOUT_D,
};

static const u32 tpm_default_durations[3] = {
    TPM_DEFAULT_DURATION_SHORT,
    TPM_DEFAULT_DURATION_MEDIUM,
    TPM_DEFAULT_DURATION_LONG,
};

/* determined values */
static u32 tpm_default_dur[3];
static u32 tpm_default_to[4];

static u32 crb_cmd_size;
static void *crb_cmd;
static u32 crb_resp_size;
static void *crb_resp;


/*
 * Waits until an expected value can be read in the register or timeout
 */
static u32 wait_reg8(u8* reg, u32 time, u8 mask, u8 expect)
{
    u32 rc = 0;
	struct predicate_args arg = {
		.reg = reg,
		.expect = expect,
		.mask = mask
	};
	for (u32 i=0; i<time/200; i++){
		if(reg_predicate(&arg)){
			rc = 1;
			break;
		}
	}
    return rc ? 0 : -1;
}

static u32 tis_wait_access(u8 locty, u32 time, u8 mask, u8 expect)
{
    return wait_reg8(TIS_REG(locty, TIS_REG_ACCESS), time, mask, expect);
}

static u32 tis_wait_sts(u8 locty, u32 time, u8 mask, u8 expect)
{
    return wait_reg8(TIS_REG(locty, TIS_REG_STS), time, mask, expect);
}

static u32 crb_wait_reg(u8 locty, u16 reg, u32 time, u8 mask, u8 expect)
{
    return wait_reg8(CRB_REG(locty, reg), time, mask, expect);
}

/* if device is not there, return '0', '1' otherwise */
static u32 tis_probe(void)
{
    /* Wait for the interface to report it's ready */
    u32 rc = tis_wait_access(0, TIS_DEFAULT_TIMEOUT_A,
                             TIS_ACCESS_TPM_REG_VALID_STS,
                             TIS_ACCESS_TPM_REG_VALID_STS);
    if (rc)
        return 0;

    u32 didvid = readl(TIS_REG(0, TIS_REG_DID_VID));

    if ((didvid != 0) && (didvid != 0xffffffff))
        rc = 1;

    /* TPM 2 has an interface register */
    u32 ifaceid = readl(TIS_REG(0, TIS_REG_IFACE_ID));

    if ((ifaceid & 0xf) != 0xf) {
        if ((ifaceid & 0xf) == 1) {
            /* CRB is active; no TIS */
            return 0;
        }
        if ((ifaceid & (1 << 13)) == 0) {
            /* TIS cannot be selected */
            return 0;
        }
        /* write of 0 to bits 17-18 selects TIS */
        write_l(TIS_REG(0, TIS_REG_IFACE_ID), 0);
        /* since we only support TIS, we lock it */
        write_l(TIS_REG(0, TIS_REG_IFACE_ID), (1 << 19));
    }

    return rc;
}

static TPMVersion tis_get_tpm_version(void)
{
    u32 reg = readl(TIS_REG(0, TIS_REG_IFACE_ID));

    /*
     * FIFO interface as defined in TIS1.3 is active
     * Interface capabilities are defined in TIS_REG_INTF_CAPABILITY
     */
    if ((reg & 0xf) == 0xf) {
        reg = readl(TIS_REG(0, TIS_REG_INTF_CAPABILITY));
        /* Interface 1.3 for TPM 2.0 */
        if (((reg >> 28) & 0x7) == 3)
            return TPM_VERSION_2;
    }
    /* FIFO interface as defined in PTP for TPM 2.0 is active */
    else if ((reg & 0xf) == 0) {
        return TPM_VERSION_2;
    }

    return TPM_VERSION_1_2;
}

static void init_timeout(int driver)
{
    if (tpm_drivers[driver].durations == NULL) {
        u32 *durations = tpm_default_dur;
        memcpy(durations, tpm_default_durations,
               sizeof(tpm_default_durations));
        tpm_drivers[driver].durations = durations;
    }

    if (tpm_drivers[driver].timeouts == NULL) {
        u32 *timeouts = tpm_default_to;
        memcpy(timeouts, tis_default_timeouts,
               sizeof(tis_default_timeouts));
        tpm_drivers[driver].timeouts = timeouts;
    }
}

/*
 * Indicate to the TPM we are using the TIS interface
 */
static u32 tis_init(void)
{

    write_b(TIS_REG(0, TIS_REG_INT_ENABLE), 0);

    init_timeout(TIS_DRIVER_IDX);

    return 1;
}


static void set_timeouts(u32 timeouts[4], u32 durations[3])
{

    u32 *tos = tpm_drivers[TIS_DRIVER_IDX].timeouts;
    u32 *dus = tpm_drivers[TIS_DRIVER_IDX].durations;

    if (tos && tos != tis_default_timeouts && timeouts)
        memcpy(tos, timeouts, 4 * sizeof(u32));
    if (dus && dus != tpm_default_durations && durations)
        memcpy(dus, durations, 3 * sizeof(u32));
}

/*
 * Activate locality using TIS interface
 */
static u32 tis_activate(u8 locty)
{

    u32 rc = 0;
    u8 acc;
    int l;
    u32 timeout_b = tpm_drivers[TIS_DRIVER_IDX].timeouts[TIS_TIMEOUT_TYPE_B];

    if (!(readb(TIS_REG(locty, TIS_REG_ACCESS)) &
          TIS_ACCESS_ACTIVE_LOCALITY)) {
        /* release locality in use top-downwards */
        for (l = 4; l >= 0; l--)
            write_b(TIS_REG(l, TIS_REG_ACCESS),
                   TIS_ACCESS_ACTIVE_LOCALITY);
    }

    /* request access to locality */
    write_b(TIS_REG(locty, TIS_REG_ACCESS), TIS_ACCESS_REQUEST_USE);

    acc = readb(TIS_REG(locty, TIS_REG_ACCESS));
    if ((acc & TIS_ACCESS_ACTIVE_LOCALITY)) {
        write_b(TIS_REG(locty, TIS_REG_STS), TIS_STS_COMMAND_READY);
        rc = tis_wait_sts(locty, timeout_b,
                          TIS_STS_COMMAND_READY, TIS_STS_COMMAND_READY);
    }

    return rc;
}

/*
 * If we aren't currently at the desired locality, free all localities
 */
static void tis_release_all_localities(u8 locty){
	int l;
    if (!(readb(TIS_REG(locty, TIS_REG_ACCESS)) &
          TIS_ACCESS_ACTIVE_LOCALITY)) {
        /* release locality in use top-downwards */
        for (l = 4; l >= 0; l--)
            write_b(TIS_REG(l, TIS_REG_ACCESS),
                   TIS_ACCESS_ACTIVE_LOCALITY);
    }

}

/*
 * Find the current locality by iterating
 */
static u32 tis_find_active_locality(void)
{

    u8 locty;

    for (locty = 0; locty <= 4; locty++) {
        if ((readb(TIS_REG(locty, TIS_REG_ACCESS)) &
             TIS_ACCESS_ACTIVE_LOCALITY))
            return locty;
    }

    tis_activate(0);

    return 0;
}

static u32 tis_ready(void)
{

    u32 rc = 0;
    u8 locty = tis_find_active_locality();
    u32 timeout_b = tpm_drivers[TIS_DRIVER_IDX].timeouts[TIS_TIMEOUT_TYPE_B];

    write_b(TIS_REG(locty, TIS_REG_STS), TIS_STS_COMMAND_READY);
    rc = tis_wait_sts(locty, timeout_b,
                      TIS_STS_COMMAND_READY, TIS_STS_COMMAND_READY);

    return rc;
}

/*
 * Sends data on the current active locality.
 * - Data is sent when TPM signals it is ready to receive (burst variable),
 * - while there is data to send,
 * - or until a timeout.
 * Taken from SeaBIOS
 */
static u32 tis_senddata(const u8 *const data, u32 len)
{

    u32 rc = 0;
    u32 offset = 0;
    u32 end_loop = 0;
    u16 burst = 0;
    u8 locty = tis_find_active_locality();
    u32 timeout_d = tpm_drivers[TIS_DRIVER_IDX].timeouts[TIS_TIMEOUT_TYPE_D];

    do {
        while (burst == 0) {
               burst = readl(TIS_REG(locty, TIS_REG_STS)) >> 8;
            if (burst == 0) {
                /*if (timer_check(end)) {*/
			/*sbi_printf("WARNING - Timeout when trying to write to TPM FIFO \n");*/
                    /*break;*/
                /*}*/
            }
        }

        if (burst == 0) {
            rc = TCG_RESPONSE_TIMEOUT;
            break;
        }

        while (1) {
            write_b(TIS_REG(locty, TIS_REG_DATA_FIFO), data[offset++]);
            burst--;

            if (burst == 0 || offset == len)
                break;
        }

        if (offset == len)
            end_loop = 1;
    } while (end_loop == 0);

    return rc;
}



/*
 * Reads data from the data channel into a buffer, until length is consumed or TPM signals data is no longer available.
 */
static u32 tis_readresp(u8 *buffer, u32 *len)
{

    u32 rc = 0;
    u32 offset = 0;
    u32 sts;
    u8 locty = tis_find_active_locality();

	
    while (offset < *len) {
        buffer[offset] = readb(TIS_REG(locty, TIS_REG_DATA_FIFO));
        offset++;
        sts = readb(TIS_REG(locty, TIS_REG_STS));
        /* data left ? */
        if ((sts & TIS_STS_DATA_AVAILABLE) == 0){
            break;
		}
    }

    *len = offset;

    return rc;
}


/*
 * Waits until the TPM signals it has received all data.
 */
static u32 tis_waitdatavalid(void)
{

    u32 rc = 0;
    u8 locty = tis_find_active_locality();
    u32 timeout_c = tpm_drivers[TIS_DRIVER_IDX].timeouts[TIS_TIMEOUT_TYPE_C];

    if (tis_wait_sts(locty, timeout_c, TIS_STS_VALID, TIS_STS_VALID) != 0)
        rc = 1;

    return rc;
}

/*
 * Waits until the TPM signals data is ready to be read (STS_DATA_AVAILABLE signal)
 */
static u32 tis_waitrespready(enum tpmDurationType to_t)
{

    u32 rc = 0;
    u8 locty = tis_find_active_locality();
    u32 timeout = tpm_drivers[TIS_DRIVER_IDX].durations[to_t];

    write_b(TIS_REG(locty ,TIS_REG_STS), TIS_STS_TPM_GO);

    if (tis_wait_sts(locty, timeout,
                     TIS_STS_DATA_AVAILABLE, TIS_STS_DATA_AVAILABLE) != 0)
        rc = 1;

    return rc;
}

#define CRB_STATE_VALID_STS 0b10000000
#define CRB_STATE_LOC_ASSIGNED 0x00000010
#define CRB_STATE_READY_MASK (CRB_STATE_VALID_STS | CRB_STATE_LOC_ASSIGNED)

/* if device is not there, return '0', '1' otherwise */
static u32 crb_probe(void)
{

    /* Wait for the interface to report it's ready */
    u32 rc = crb_wait_reg(0, CRB_REG_LOC_STATE, TIS2_DEFAULT_TIMEOUT_D,
                          CRB_STATE_READY_MASK, CRB_STATE_VALID_STS);
    if (rc)
        return 0;

    u32 ifaceid = readl(CRB_REG(0, CRB_REG_INTF_ID));

    if ((ifaceid & 0xf) != 0xf) {
        if ((ifaceid & 0xf) == 1) {
            /* CRB is active */
        } else if ((ifaceid & (1 << 14)) == 0) {
            /* CRB cannot be selected */
            return 0;
        }
        /* write of 1 to bits 17-18 selects CRB */
        write_l(CRB_REG(0, CRB_REG_INTF_ID), (1 << 17));
        /* lock it */
        write_l(CRB_REG(0, CRB_REG_INTF_ID), (1 << 19));
    }

    /* no support for 64 bit addressing yet */
    if (readl(CRB_REG(0, CRB_REG_CTRL_CMD_HADDR)))
        return 0;

    u64 addr = readq(CRB_REG(0, CRB_REG_CTRL_RSP_ADDR));
    if (addr > 0xffffffff)
        return 0;

    return 1;
}

static TPMVersion crb_get_tpm_version(void)
{
    /* CRB is supposed to be TPM 2.0 only */
    return TPM_VERSION_2;
}

static u32 crb_init(void)
{

    crb_cmd = (void*) (uintptr_t) readl(CRB_REG(0, CRB_REG_CTRL_CMD_LADDR));
    crb_cmd_size = readl(CRB_REG(0, CRB_REG_CTRL_CMD_SIZE));
    crb_resp = (void*)(uintptr_t)readl(CRB_REG(0, CRB_REG_CTRL_RSP_ADDR));
    crb_resp_size = readl(CRB_REG(0, CRB_REG_CTRL_RSP_SIZE));

    init_timeout(CRB_DRIVER_IDX);

    return 0;
}

static u32 crb_activate(u8 locty)
{

    write_b(CRB_REG(locty, CRB_REG_LOC_CTRL), 1);

    return 0;
}

static u32 crb_find_active_locality(void)
{

    return 0;
}

#define CRB_CTRL_REQ_CMD_READY 0b1
#define CRB_START_INVOKE 0b1
#define CRB_CTRL_STS_ERROR 0b1

static u32 crb_ready(void)
{

    u32 rc = 0;
    u8 locty = crb_find_active_locality();
    u32 timeout_c = tpm_drivers[CRB_DRIVER_IDX].timeouts[TIS_TIMEOUT_TYPE_C];

    write_l(CRB_REG(locty, CRB_REG_CTRL_REQ), CRB_CTRL_REQ_CMD_READY);
    rc = crb_wait_reg(locty, CRB_REG_CTRL_REQ, timeout_c,
                      CRB_CTRL_REQ_CMD_READY, 0);

    return rc;
}

static u32 crb_senddata(const u8 *const data, u32 len)
{

    if (len > crb_cmd_size)
        return 1;

    u8 locty = crb_find_active_locality();
    memcpy(crb_cmd, data, len);
    write_l(CRB_REG(locty, CRB_REG_CTRL_START), CRB_START_INVOKE);

    return 0;
}

static u32 crb_readresp(u8 *buffer, u32 *len)
{

    u8 locty = crb_find_active_locality();
    if (readl(CRB_REG(locty, CRB_REG_CTRL_STS)) & CRB_CTRL_STS_ERROR)
        return 1;

    if (*len < 6)
        return 1;

    memcpy(buffer, crb_resp, 6);
    u32 expected = be32_to_cpu(*(u32 *) &buffer[2]);
    if (expected < 6)
        return 1;

    *len = (*len < expected) ? *len : expected;

    memcpy(buffer + 6, crb_resp + 6, *len - 6);

    return 0;
}


static u32 crb_waitdatavalid(void)
{
    return 0;
}

static u32 crb_waitrespready(enum tpmDurationType to_t)
{

    u32 rc = 0;
    u8 locty = crb_find_active_locality();
    u32 timeout = tpm_drivers[CRB_DRIVER_IDX].durations[to_t];

    rc = crb_wait_reg(locty, CRB_REG_CTRL_START, timeout,
                      CRB_START_INVOKE, 0);

    return rc;
}

struct tpm_driver tpm_drivers[TPM_NUM_DRIVERS] = {
	[TIS_DRIVER_IDX]= {
		.timeouts      = NULL,
		.durations     = NULL,
		.set_timeouts  = set_timeouts,
		.probe         = tis_probe,
		.get_tpm_version = tis_get_tpm_version,
		.init          = tis_init,
		.activate      = tis_activate,
		.ready         = tis_ready,
		.senddata      = tis_senddata,
		.readresp      = tis_readresp,
		.waitdatavalid = tis_waitdatavalid,
		.waitrespready = tis_waitrespready,
	},
	[CRB_DRIVER_IDX] = {
			.timeouts      = NULL,
			.durations     = NULL,
			.set_timeouts  = set_timeouts,
			.probe         = crb_probe,
			.get_tpm_version = crb_get_tpm_version,
			.init          = crb_init,
			.activate      = crb_activate,
			.ready         = crb_ready,
			.senddata      = crb_senddata,
			.readresp      = crb_readresp,
			.waitdatavalid = crb_waitdatavalid,
			.waitrespready = crb_waitrespready,
		}
};

static u8 TPMHW_driver_to_use = TPM_INVALID_DRIVER;

/*
 * Checks if the TPM is physically present. Checks which interface it supports (TIS first)
 * Initialize the given interface if TPM responds
 */
TPMVersion
tpmhw_probe(void)
{

    unsigned int i;
    for (i = 0; i < TPM_NUM_DRIVERS; i++) {
        struct tpm_driver td = tpm_drivers[i];

        if (td.probe() != 0) {
            td.init();
            TPMHW_driver_to_use = i;
            return td.get_tpm_version();
        }
    }
    return TPM_VERSION_NONE;
}

/*
 * Helper function to ensure the TPM is reachable by one of the support interfaces (and that an interface has been init)
 */
int
tpmhw_is_present(void)
{
    return TPMHW_driver_to_use != TPM_INVALID_DRIVER;
}

int tpm_read_attestation(u8 locty,
               void **respbuffer, u32 *respbufferlen,
               enum tpmDurationType to_t)
{
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER){
        return -1;
	}
	/**respbuffer = sbi_scratch_thishart_offset_ptr(sbi_scratch_alloc_offset(namesize));*/

	return 0;

}

int tpmhw_read_dynamic_buffer(
		void *respbuffer, u32 *respbufferlen,
               enum tpmDurationType to_t){
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER){
        return -1;
	}

	u32 sizeVal = *respbufferlen;

    struct tpm_driver *td = &tpm_drivers[TPMHW_driver_to_use];

    u32 irc = td->waitrespready(to_t);
    if (irc != 0){
        return -1;
	}


	irc = td->readresp(respbuffer, respbufferlen);
    if (irc != 0 ||
        *respbufferlen < sizeVal){
        return -1;}

return 0;
}


/*
 * Sends data to the TPM and collects response.
 * Data size is sotred in the request header (req)
 * Response size is explicitely given in respbufferlen
 */
int
tpmhw_transmit(u8 locty, struct tpm_req_header *req,
               void *respbuffer, u32 *respbufferlen,
               enum tpmDurationType to_t)
{
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER){
        return -1;
    }

    struct tpm_driver *td = &tpm_drivers[TPMHW_driver_to_use];

    u32 irc = td->activate(locty);
    if (irc != 0) {
        /* tpm could not be activated */
        return -2;
    }

    irc = td->senddata((void*)req, be32_to_cpu(req->totlen));
    if (irc != 0)
        return -3;

    irc = td->waitdatavalid();
    if (irc != 0)
        return -4;

    irc = td->waitrespready(to_t);
    if (irc != 0)
        return -5;

    irc = td->readresp(respbuffer, respbufferlen);
    if (irc != 0 ||
        *respbufferlen < sizeof(struct tpm_rsp_header)){
        return -6;}


    td->ready();

    return 0;
}
/*
 * TPM_HASH_START command is realized by writing to a specific register on locality 4
 */
static u32 tis_hash_start(u8 locty){
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER){
        return -1;
	}

	struct tpm_driver *td = &tpm_drivers[TPMHW_driver_to_use];

	/* This requests access to locality 4 */
	tis_release_all_localities(locty);
	timer_delay_loop(10000);
	
	u32 irc = td->activate(locty);
	if (irc != 0) {
        /*[> tpm could not be activated <]*/
		return -1;
	}
	write_b(TIS_REG(locty, TIS_REG_HASH_START), 1);

	return 0;
}

/*
 * TPM_HASH_DATA is realized by sending data through the data stream as for other control channel commands.
 * We are redefining the senddata function as we aren't ensured the TPM is started.
 */
static u32 tis_hash_senddata(u8 locty, const u8 * const data, u32 len){

	/*We are about to send data on a locality other than 4 */
	u8 rc = 0;
	u32 offset = 0;
	u8 end_loop = 0;
	u64 elem = 0;
	do {
		
		//If we can write 8 bytes in a single go, we do, otherwise we write byte by byte.
		if (offset + sizeof(u64)/sizeof(u8) < len) {
			elem =  ((u64 *) data)[offset];
			offset +=sizeof(u64)/sizeof(u8);
			write_l(TIS_REG(locty, TIS_HASH_DATA_FIFO), elem);
		}
		else{
			write_b(TIS_REG(locty, TIS_HASH_DATA_FIFO), data[offset++]);
		}
		/* Blocking wait */
		timer_delay_loop(10000);

		if (offset==len)
			end_loop = 1;
	
	} while (end_loop == 0);
	
	return rc;
}

u32 tpm_hash_start_loc4(void){
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER || TPMHW_driver_to_use == CRB_DRIVER_IDX){
        return -1;
	}

	return tis_hash_start(4);
}


/*
 * TPM_HASH_END is realized by writing to the appropriate register in locality 4.
 */
static u32 tis_hash_end(u8 locty){
	write_b(TIS_REG(locty, TIS_REG_HASH_END), 1);
	timer_delay_loop(1000000);
	/*According to the TCG specification, locality is released after DRTM ops*/
	return 0;
}


u32 tpm_hash_end_loc4(void){
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER || TPMHW_driver_to_use == CRB_DRIVER_IDX){
        return -1;
	}

	return tis_hash_end(4);

}

int tpm_senddata_loc4(const u8* data, u32 len){
    if (TPMHW_driver_to_use == TPM_INVALID_DRIVER || TPMHW_driver_to_use == CRB_DRIVER_IDX){
	//We are only working on TIS for DRTM.
        return -1;
	}
	return tis_hash_senddata(4, data, len);
}

void
tpmhw_set_timeouts(u32 timeouts[4], u32 durations[3])
{

    struct tpm_driver *td = &tpm_drivers[TPMHW_driver_to_use];
    td->set_timeouts(timeouts, durations);
}
