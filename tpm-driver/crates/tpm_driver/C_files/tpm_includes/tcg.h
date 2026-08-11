/* TCG specification header file for TCG compliance
 *
 * Taken from the SeaBios specification (at https://github.com/coreboot/seabios)
 *
 * Adapted for OpenSBI. Everything related to ACPI tables has been removed for simplicity purposes.
 */
#ifndef STD_TCG_H
#define STD_TCG_H

#include <stdint.h>
#include "TpmTypes.h"

#define SHA1_BUFSIZE                20
#define SHA256_BUFSIZE              32
#define SHA384_BUFSIZE              48
#define SHA512_BUFSIZE              64
#define SM3_256_BUFSIZE             32
#define SHA3_256_BUFSIZE            32
#define SHA3_384_BUFSIZE            48
#define SHA3_512_BUFSIZE            64


/****************************************************************
 * 16bit BIOS interface
 ****************************************************************/

/* Define for section 12.3 */
#define TCG_PC_OK                       0x0
#define TCG_PC_TPMERROR                 0x1
#define TCG_PC_LOGOVERFLOW              0x2
#define TCG_PC_UNSUPPORTED              0x3


//#define TPM_ALG_SHA                     0x4

#define TCG_MAGIC                       0x41504354L
#define TCG_VERSION_MAJOR               1
#define TCG_VERSION_MINOR               2

#define TPM_OK                          0x0
#define TPM_RET_BASE                    0x1
#define TCG_GENERAL_ERROR               (TPM_RET_BASE + 0x0)
#define TCG_TPM_IS_LOCKED               (TPM_RET_BASE + 0x1)
#define TCG_NO_RESPONSE                 (TPM_RET_BASE + 0x2)
#define TCG_INVALID_RESPONSE            (TPM_RET_BASE + 0x3)
#define TCG_INVALID_ACCESS_REQUEST      (TPM_RET_BASE + 0x4)
#define TCG_FIRMWARE_ERROR              (TPM_RET_BASE + 0x5)
#define TCG_INTEGRITY_CHECK_FAILED      (TPM_RET_BASE + 0x6)
#define TCG_INVALID_DEVICE_ID           (TPM_RET_BASE + 0x7)
#define TCG_INVALID_VENDOR_ID           (TPM_RET_BASE + 0x8)
#define TCG_UNABLE_TO_OPEN              (TPM_RET_BASE + 0x9)
#define TCG_UNABLE_TO_CLOSE             (TPM_RET_BASE + 0xa)
#define TCG_RESPONSE_TIMEOUT            (TPM_RET_BASE + 0xb)
#define TCG_INVALID_COM_REQUEST         (TPM_RET_BASE + 0xc)
#define TCG_INVALID_ADR_REQUEST         (TPM_RET_BASE + 0xd)
#define TCG_WRITE_BYTE_ERROR            (TPM_RET_BASE + 0xe)
#define TCG_READ_BYTE_ERROR             (TPM_RET_BASE + 0xf)
#define TCG_BLOCK_WRITE_TIMEOUT         (TPM_RET_BASE + 0x10)
#define TCG_CHAR_WRITE_TIMEOUT          (TPM_RET_BASE + 0x11)
#define TCG_CHAR_READ_TIMEOUT           (TPM_RET_BASE + 0x12)
#define TCG_BLOCK_READ_TIMEOUT          (TPM_RET_BASE + 0x13)
#define TCG_TRANSFER_ABORT              (TPM_RET_BASE + 0x14)
#define TCG_INVALID_DRV_FUNCTION        (TPM_RET_BASE + 0x15)
#define TCG_OUTPUT_BUFFER_TOO_SHORT     (TPM_RET_BASE + 0x16)
#define TCG_FATAL_COM_ERROR             (TPM_RET_BASE + 0x17)
#define TCG_INVALID_INPUT_PARA          (TPM_RET_BASE + 0x18)
#define TCG_TCG_COMMAND_ERROR           (TPM_RET_BASE + 0x19)
#define TCG_INTERFACE_SHUTDOWN          (TPM_RET_BASE + 0x20)
//define TCG_PC_UNSUPPORTED             (TPM_RET_BASE + 0x21)
#define TCG_PC_TPM_NOT_PRESENT          (TPM_RET_BASE + 0x22)
#define TCG_PC_TPM_DEACTIVATED          (TPM_RET_BASE + 0x23)

/* interrupt identifiers (al register) */
enum irq_ids {
    TCG_StatusCheck = 0,
    TCG_HashLogExtendEvent = 1,
    TCG_PassThroughToTPM = 2,
    TCG_ShutdownPreBootInterface = 3,
    TCG_HashLogEvent = 4,
    TCG_HashAll = 5,
    TCG_TSS = 6,
    TCG_CompactHashLogExtendEvent = 7,
};

/* Input and Output blocks for the TCG BIOS commands */

struct hleei_short
{
    u16   ipblength;
    u16   reserved;
    const void *hashdataptr;
    u32   hashdatalen;
    u32   pcrindex;
    const void *logdataptr;
    u32   logdatalen;
} __attribute__((packed));

struct hleei_long
{
    u16   ipblength;
    u16   reserved;
    void *hashdataptr;
    u32   hashdatalen;
    u32   pcrindex;
    u32   reserved2;
    void *logdataptr;
    u32   logdatalen;
} __attribute__((packed));

struct hleeo
{
    u16    opblength;
    u16    reserved;
    u32    eventnumber;
    u8     digest[SHA1_BUFSIZE];
} __attribute__((packed));

struct pttti
{
    u16    ipblength;
    u16    reserved;
    u16    opblength;
    u16    reserved2;
    u8     tpmopin[0];
} __attribute__((packed));

struct pttto
{
    u16    opblength;
    u16    reserved;
    u8     tpmopout[0];
};

struct hlei
{
    u16    ipblength;
    u16    reserved;
    const void  *hashdataptr;
    u32    hashdatalen;
    u32    pcrindex;
    u32    logeventtype;
    const void  *logdataptr;
    u32    logdatalen;
} __attribute__((packed));

struct hleo
{
    u16    opblength;
    u16    reserved;
    u32    eventnumber;
} __attribute__((packed));

struct hai
{
    u16    ipblength;
    u16    reserved;
    const void  *hashdataptr;
    u32    hashdatalen;
    u32    algorithmid;
} __attribute__((packed));

struct ti
{
    u16    ipblength;
    u16    reserved;
    u16    opblength;
    u16    reserved2;
    u8     tssoperandin[0];
} __attribute__((packed));

struct to
{
    u16    opblength;
    u16    reserved;
    u8     tssoperandout[0];
} __attribute__((packed));

struct pcpes
{
    u32    pcrindex;
    u32    eventtype;
    u8     digest[SHA1_BUFSIZE];
    u32    eventdatasize;
    u8     event[0];
} __attribute__((packed));


/****************************************************************
 * TPM v1.2 hardware commands
 ****************************************************************/

#define TPM_ORD_SelfTestFull             0x00000050
#define TPM_ORD_ForceClear               0x0000005d
#define TPM_ORD_GetCapability            0x00000065
#define TPM_ORD_PhysicalEnable           0x0000006f
#define TPM_ORD_PhysicalDisable          0x00000070
#define TPM_ORD_SetOwnerInstall          0x00000071
#define TPM_ORD_PhysicalSetDeactivated   0x00000072
#define TPM_ORD_SetTempDeactivated       0x00000073
#define TPM_ORD_Startup                  0x00000099
#define TPM_ORD_PhysicalPresence         0x4000000a
#define TPM_ORD_Extend                   0x00000014
#define TSC_ORD_ResetEstablishmentBit    0x4000000b

#define TPM_ST_CLEAR                     0x0001
#define TPM_ST_STATE                     0x0002
#define TPM_ST_DEACTIVATED               0x0003

#define TPM_PP_CMD_ENABLE                0x0020
#define TPM_PP_PRESENT                   0x0008
#define TPM_PP_NOT_PRESENT_LOCK          0x0014

/* TPM command error codes */
#define TPM_INVALID_POSTINIT             0x26
#define TPM_BAD_LOCALITY                 0x3d

/* TPM command tags */
#define TPM_TAG_RQU_CMD                  0x00c1
#define TPM_TAG_RQU_AUTH1_CMD            0x00c2
#define TPM_TAG_RQU_AUTH2_CMD            0x00c3

struct tpm_req_header {
    u16    tag;
    u32    totlen;
    u32    ordinal;
} __attribute__((packed));

struct tpm_rsp_header {
    u16    tag;
    u32    totlen;
    u32    errcode;
} __attribute__((packed));

struct tpm_req_extend {
    struct tpm_req_header hdr;
    u32    pcrindex;
    u8     digest[SHA1_BUFSIZE];
} __attribute__((packed));

struct tpm_rsp_extend {
    struct tpm_rsp_header hdr;
    u8     digest[SHA1_BUFSIZE];
} __attribute__((packed));

struct tpm_req_getcap {
    struct tpm_req_header hdr;
    u32    capArea;
    u32    subCapSize;
    u32    subCap;
} __attribute__((packed));

#define TPM_CAP_FLAG     0x04
#define TPM_CAP_PROPERTY 0x05
#define TPM_CAP_FLAG_PERMANENT   0x108
#define TPM_CAP_FLAG_VOLATILE    0x109
#define TPM_CAP_PROP_OWNER       0x111
#define TPM_CAP_PROP_TIS_TIMEOUT 0x115
#define TPM_CAP_PROP_DURATION    0x120

struct tpm_permanent_flags {
    u16    tag;
    u8     flags[20];
} __attribute__((packed));

enum permFlagsIndex {
    PERM_FLAG_IDX_DISABLE = 0,
    PERM_FLAG_IDX_OWNERSHIP,
    PERM_FLAG_IDX_DEACTIVATED,
    PERM_FLAG_IDX_READPUBEK,
    PERM_FLAG_IDX_DISABLEOWNERCLEAR,
    PERM_FLAG_IDX_ALLOW_MAINTENANCE,
    PERM_FLAG_IDX_PHYSICAL_PRESENCE_LIFETIME_LOCK,
    PERM_FLAG_IDX_PHYSICAL_PRESENCE_HW_ENABLE,
    PERM_FLAG_IDX_PHYSICAL_PRESENCE_CMD_ENABLE,
};

struct tpm_res_getcap_perm_flags {
    struct tpm_rsp_header hdr;
    u32    size;
    struct tpm_permanent_flags perm_flags;
} __attribute__((packed));

struct tpm_stclear_flags {
    u16    tag;
    u8     flags[5];
} __attribute__((packed));

#define STCLEAR_FLAG_IDX_DEACTIVATED 0
#define STCLEAR_FLAG_IDX_DISABLE_FORCE_CLEAR 1
#define STCLEAR_FLAG_IDX_PHYSICAL_PRESENCE 2
#define STCLEAR_FLAG_IDX_PHYSICAL_PRESENCE_LOCK 3
#define STCLEAR_FLAG_IDX_GLOBAL_LOCK 4

struct tpm_res_getcap_stclear_flags {
    struct tpm_rsp_header hdr;
    u32    size;
    struct tpm_stclear_flags stclear_flags;
} __attribute__((packed));

struct tpm_res_getcap_ownerauth {
    struct tpm_rsp_header hdr;
    u32    size;
    u8     flag;
} __attribute__((packed));

struct tpm_res_getcap_timeouts {
    struct tpm_rsp_header hdr;
    u32    size;
    u32    timeouts[4];
} __attribute__((packed));

struct tpm_res_getcap_durations {
    struct tpm_rsp_header hdr;
    u32    size;
    u32    durations[3];
} __attribute__((packed));

struct tpm_res_sha1start {
    struct tpm_rsp_header hdr;
    u32    max_num_bytes;
} __attribute__((packed));

struct tpm_res_sha1complete {
    struct tpm_rsp_header hdr;
    u8     hash[20];
} __attribute__((packed));


/****************************************************************
 * TPM v2.0 hardware commands
 ****************************************************************/

#define TPM2_NO                     0
#define TPM2_YES                    1

#define TPM2_SU_CLEAR               0x0000
#define TPM2_SU_STATE               0x0001

#define TPM2_RH_OWNER               0x40000001
#define TPM2_RS_PW                  0x40000009
#define TPM2_RH_ENDORSEMENT         0x4000000b
#define TPM2_RH_PLATFORM            0x4000000c

#define TPM2_ALG_SHA1               0x0004
#define TPM2_ALG_HMAC 				0x0005
#define TPM2_ALG_SHA256             0x000b
#define TPM2_ALG_SHA384             0x000c
#define TPM2_ALG_SHA512             0x000d
#define TPM2_ALG_SM3_256            0x0012
#define TPM2_ALG_SHA3_256           0x0027
#define TPM2_ALG_SHA3_384           0x0028
#define TPM2_ALG_SHA3_512           0x0029

#define TPM2_ALG_SHA1_FLAG          (1 << 0)
#define TPM2_ALG_SHA256_FLAG        (1 << 1)
#define TPM2_ALG_SHA384_FLAG        (1 << 2)
#define TPM2_ALG_SHA512_FLAG        (1 << 3)
#define TPM2_ALG_SM3_256_FLAG       (1 << 4)
#define TPM2_ALG_SHA3_256_FLAG      (1 << 5)
#define TPM2_ALG_SHA3_384_FLAG      (1 << 6)
#define TPM2_ALG_SHA3_512_FLAG      (1 << 7)

/* TPM 2 command tags */
#define TPM2_ST_NO_SESSIONS         0x8001
#define TPM2_ST_SESSIONS            0x8002

/* TPM 2 commands */
#define TPM2_CC_HierarchyControl    0x121
#define TPM2_CC_Clear               0x126
#define TPM2_CC_ClearControl        0x127
#define TPM2_CC_HierarchyChangeAuth 0x129
#define TPM2_CC_PCR_Allocate        0x12b
#define TPM2_CC_SelfTest            0x143
#define TPM2_CC_Startup             0x144
#define TPM2_CC_Shutdown            0x145
#define TPM2_CC_StirRandom          0x146
#define TPM2_CC_Quote 				0x158
#define TPM2_CC_GetCapability       0x17a
#define TPM2_CC_GetRandom           0x17b
#define TPM2_CC_PCRRead 			0x17e
#define TPM2_CC_PCR_Extend          0x182

/* TPM 2 error codes */
#define TPM2_RC_INITIALIZE          0x100

/* TPM 2 Capabilities */
#define TPM2_CAP_PCRS               0x00000005

/* TPM 2 data structures */

struct tpm2_req_stirrandom {
    struct tpm_req_header hdr;
    u16 size;
    u64 stir;
} __attribute__((packed));

struct tpm2_req_getrandom {
    struct tpm_req_header hdr;
    u16 bytesRequested;
} __attribute__((packed));

struct tpm2b_20 {
    u16 size;
    u8 buffer[20];
} __attribute__((packed));

struct tpm2_res_getrandom {
    struct tpm_rsp_header hdr;
    struct tpm2b_20 rnd;
} __attribute__((packed));


struct tpm2_authArea{
	u32 authSize;
	void* authArea;
}__attribute__((packed));

struct tpm2_authblock {
    u32 handle;
    u16 noncesize;  /* always 0 */
    u8 contsession; /* always TPM2_YES */
    u16 pwdsize;    /* always 0 */
} __attribute__((packed));

struct tpm2_authblock_with_pwd{
    u32 handle;
    u16 noncesize;  /* always 0 */
    u8 contsession; /* always TPM2_YES */
    u16 pwdsize;    /* always 64 */
	u8 pwd[64];
}__attribute__((packed));


struct tpm2_req_hierarchychangeauth {
    struct tpm_req_header hdr;
    u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
    struct tpm2b_20 newAuth;
} __attribute__((packed));

struct tpm2_req_extend {
    struct tpm_req_header hdr;
    u32 pcrindex;
    u32 authblocksize;
    struct tpm2_authblock authblock;
    u8 digest[0];
} __attribute__((packed));

struct tpmt_sig_scheme {
	u16 signAlg;
	u16 tpmu_alg_scheme;
} __attribute__((packed));


struct tpm2_req_clearcontrol {
    struct tpm_req_header hdr;
    u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
    u8 disable;
} __attribute__((packed));

struct tpm2_req_clear {
    struct tpm_req_header hdr;
    u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
} __attribute__((packed));

struct tpm2_req_hierarchycontrol {
    struct tpm_req_header hdr;
    u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
    u32 enable;
    u8 state;
} __attribute__((packed));

struct tpm2_req_getcapability {
    struct tpm_req_header hdr;
    u32 capability;
    u32 property;
    u32 propertycount;
} __attribute__((packed));

struct tpm2_res_getcapability {
    struct tpm_rsp_header hdr;
    u8 moreData;
    u32 capability;
    u8 data[0]; /* capability dependent data */
} __attribute__((packed));

struct tpms_pcr_selection {
    u16 hashAlg;
    u8 sizeOfSelect;
    u8 pcrSelect[0];
} __attribute__((packed));

struct tpml_pcr_selection {
    u32 count;
    struct tpms_pcr_selection selections[0];
} __attribute__((packed));

struct tpm2_req_pcr_allocate {
    struct tpm_req_header hdr;
    u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
    u32 count;
    u8 tpms_pcr_selections[0];
} __attribute__((packed));


struct tpm2_req_quote {
	struct tpm_req_header hdr;
	u32 signKeyHandle;
	u32 authblocksize;
	struct tpm2_authblock_with_pwd authblock;
	u16 qualifyingDatasize;
	u16 algSchemeName;
	struct tpml_pcr_selection pcrSel;
} __attribute__((packed));

struct tpm2b_digest {
	u16 size;
	u8 buffer[0];
} __attribute__((packed));

struct tpm2b_label {
	u16 size; //always 16
	u8 buffer[16];
}__attribute__((packed));

struct tpms_derive {
	struct tpm2b_label label;
	struct tpm2b_label context;
}__attribute__((packed));

struct tpm2b_auth {
	u16 size;
	u8 buffer[64];
} __attribute__((packed));

struct tpm2b_sensitive_data {
	u16 size;
	u8 buffer[64];
} __attribute__((packed));

struct tpm2b_sensitive_data_rsa {
	u16 size; //0
	u8 buffer[0];
} __attribute__((packed));


struct tpmi_alg_keyedhash_scheme {
	u16 hashAlg; //TPM_ALG_HMAC or TPM_ALG_XOR
}__attribute__((packed));


typedef struct tpmi_alg_keyedhash_scheme tpms_scheme_hmac;

struct tpms_scheme_xor {
	u16 hashAlg;
	u16 kdf;
}__attribute__((packed));

struct tpmu_scheme_keyedhash  {
	tpms_scheme_hmac hmac;
}__attribute__((packed));

struct tpmt_keyedhash_parms {
	struct tpmi_alg_keyedhash_scheme scheme;
	struct tpmu_scheme_keyedhash details;
}__attribute__((packed));

struct tpmt_keyedhash_parms_xor {
	struct tpmi_alg_keyedhash_scheme scheme;
	struct tpms_scheme_xor details;
}__attribute__((packed));

struct tpms_sensitive_create {
	struct tpm2b_auth userAuth;
	struct tpm2b_sensitive_data data;
} __attribute__((packed));

struct tpms_sensitive_create_rsa {
	struct tpm2b_auth userAuth;
	struct tpm2b_sensitive_data_rsa data;
}__attribute__((packed));

struct tpm2b_sensitive_create {
	u16 size;
	struct tpms_sensitive_create sensitive;
} __attribute__((packed));

struct tpm2b_sensitive_create_rsa {
	u16 size;
	struct tpms_sensitive_create_rsa sensitive;
} __attribute__((packed));

struct tpma_object {
	u32 attributes;
}__attribute__((packed));

struct tpms_keyedhash_parms {
	struct tpmt_keyedhash_parms scheme;
} __attribute__((packed));

struct tpms_keyedhash_parms_xor {
	struct tpmt_keyedhash_parms_xor scheme;
} __attribute__((packed));


struct tpmt_rsa_scheme {
	u16 scheme; //Shall be TPM_ALG_RSASSA
	u16 details;
}__attribute__((packed));


struct tpms_rsa_parms{
	u16 symmetricAlg; //Shall be TPM_ALG_NULL as this object is not a parent
	struct tpmt_rsa_scheme scheme;
	u16 keyBits;
	u32 publicExponent;
}__attribute__((packed));

struct tpmt_sym_def_object {
	u16 algName;
	u16 keyBits;
	u16 algMode;
}__attribute__((packed));

struct tpms_rsa_parms_parent{
	struct tpmt_sym_def_object symmetricAlg;
	u16 nullAlg; //Shall be TPM_ALG_NULL as this object is a storage parent
	u16 keyBits;
	u32 publicExponent;
}__attribute__((packed));

struct tpmt_public_xor {
	u16 hashCategory; //TPM_ALG_KEYEDHASH from TpmTypes.h
	u16 namingAlg; //by default, let's put TPM_ALG_SHA256 (won't allow for TPM_ALG_NULL)
	struct tpma_object objectAttributes;
	struct tpm2b_digest authPolicy;
	struct tpms_keyedhash_parms_xor keyedHash_parms;
	struct tpm2b_digest keyedHash_unique;
}__attribute__((packed));

typedef struct tpmt_public_xor tpmt_public ;

struct tpm2b_public_xor {
	u16 size;
	struct tpmt_public_xor publicArea;
}__attribute__((packed));

struct tpmt_public_rsa {
	u16 hashCategory; //TPM_ALG_KEYEDHASH from TpmTypes.h
	u16 namingAlg; //by default, let's put TPM_ALG_SHA256 (won't allow for TPM_ALG_NULL)
	struct tpma_object objectAttributes;
	struct tpm2b_digest authPolicy;
	struct tpms_rsa_parms rsaParms;
	u16 public_rsa_key_buffer;
}__attribute__((packed));

struct tpmt_public_rsa_parent {
	u16 hashCategory; //TPM_ALG_RSA from TpmTypes.h
	u16 namingAlg; //by default, let's put TPM_ALG_SHA256 (won't allow for TPM_ALG_NULL)
	struct tpma_object objectAttributes;
	struct tpm2b_digest authPolicy;
	struct tpms_rsa_parms_parent rsaParms;
	u16 public_rsa_key_buffer; //Shall be size 0 for create
}__attribute__((packed));

struct tpm2b_public_rsa_parent {
	u16 size;
	struct tpmt_public_rsa_parent publicArea;
}__attribute__((packed));

struct tpm2b_public_rsa {
	u16 size;
	struct tpmt_public_rsa publicArea;
}__attribute__((packed));

struct tpm2b_public {
	u16 size;
	tpmt_public publicArea;
}__attribute__((packed));

struct tpmt_ha_sha256{
	u16 hashAlg;
	u8 digest[SHA256_DIGEST_SIZE];
}__attribute__((packed));

struct tpmt_signature_hmac_sha256 {
	u16 hashAlg;
	struct tpmt_ha_sha256 digest;
}__attribute__((packed));

struct tpms_signature_rsa {
	u16 hashAlg;
	u16 signatureSize;
	u8 signature[384];
}__attribute__((packed));

struct tpmt_signature_rsa {
	u16 signatureAlgorithm;
	struct tpms_signature_rsa sig;
}__attribute__((packed));


struct tpm2_req_createPrimary {
	struct tpm_req_header hdr;
	u32 rh_hierarchy;
	//u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock authblock;
	struct tpm2b_sensitive_create inSensitive;
	//u16 publicsize;
	struct tpm2b_public inPublic;
	//struct tpm2b_digest outsideInfo;
	//struct tpml_pcr_selection creationPcr;
} __attribute__((packed));

struct tpm2_req_createPrimaryXOR {
	struct tpm_req_header hdr;
	u32 rh_hierarchy;
	u32 authblocksize;
	struct tpm2_authblock authblock;
	struct tpm2b_sensitive_create inSensitive;
	struct tpm2b_public_xor inPublic;
}__attribute__((packed));

struct tpm2_req_createPrimaryRSA {
	struct tpm_req_header hdr;
	u32 rh_hierarchy;
	u32 authblocksize;
	struct tpm2_authblock authblock;
	struct tpm2b_sensitive_create inSensitive;
	struct tpm2b_public_rsa_parent inPublic;
}__attribute__((packed));

struct tpm2_req_createLoaded {
	struct tpm_req_header hdr;
	u32 rh_hierarchy;
	//u32 authhandle;
    u32 authblocksize;
    struct tpm2_authblock_with_pwd authblock;
	struct tpm2b_sensitive_create_rsa inSensitive;
	//u16 publicsize;
	struct tpm2b_public_rsa inPublic;
	//struct tpm2b_digest outsideInfo;
	//struct tpml_pcr_selection creationPcr;

}__attribute__((packed));

struct tpms_attest{
	u32 magic; //TPM_GENERATED_VALUE for tpm-generated values (0xff544347)
	u16 type;  //in our case, we want this to be TPMS_QUOTE_INFO (0x8018)
	u16 nameSize;
	u8 qualifiedSignerName[34];
	u16 extraDataSize;
	TPMS_CLOCK_INFO clockInfo;
	u64 firmwareVersion;
	struct tpml_pcr_selection pcrSelect;
	struct tpms_pcr_selection pcrSel;
}__attribute__((packed));

struct tpm2b_attest{
	u16 attestSize;
	struct tpms_attest attestationData;
}__attribute__((packed));

struct tpm2_quote_rsp{
	struct tpm_rsp_header hdr;
	u32 parameterSize;
	struct tpm2b_attest quoted;
}__attribute__((packed));


/****************************************************************
 * ACPI TCPA table interface
 ****************************************************************/

/* event types: 10.4.1 / table 11 */
#define EV_POST_CODE             1
#define EV_NO_ACTION             3
#define EV_SEPARATOR             4
#define EV_ACTION                5
#define EV_EVENT_TAG             6
#define EV_COMPACT_HASH         12
#define EV_IPL                  13
#define EV_IPL_PARTITION_DATA   14

struct tpm2_digest_value {
    u16 hashAlg;
    u8 hash[0]; /* size depends on hashAlg */
} __attribute__((packed));

struct tpm2_digest_values {
    u32 count;
    struct tpm2_digest_value digest[0];
} __attribute__((packed));

// Each entry in the TPM log contains: a tpm_log_header, a variable
// length digest, a tpm_log_trailer, and a variable length event.  The
// 'digest' matches what is sent to the TPM hardware via the Extend
// command.  On TPM1.2 the digest is a SHA1 hash; on TPM2.0 the digest
// contains a tpm2_digest_values struct followed by a variable number
// of tpm2_digest_value structs (as specified by the hardware via the
// TPM2_CAP_PCRS request).
struct tpm_log_header {
    u32 pcrindex;
    u32 eventtype;
    u8 digest[0];
} __attribute__((packed));

struct tpm_log_trailer {
    u32 eventdatasize;
    u8 event[0];
} __attribute__((packed));

struct TCG_EfiSpecIdEventStruct {
    u8 signature[16];
    u32 platformClass;
    u8 specVersionMinor;
    u8 specVersionMajor;
    u8 specErrata;
    u8 uintnSize;
    u32 numberOfAlgorithms;
    struct TCG_EfiSpecIdEventAlgorithmSize {
        u16 algorithmId;
        u16 digestSize;
    } digestSizes[];
    /*
    u8 vendorInfoSize;
    u8 vendorInfo[0];
    */
} __attribute__((packed));

#define TPM_TCPA_ACPI_CLASS_CLIENT 0

struct pcctes
{
    u32 eventid;
    u32 eventdatasize;
    u8  digest[SHA1_BUFSIZE];
} __attribute__((packed));

struct pcctes_romex
{
    u32 eventid;
    u32 eventdatasize;
    u16 reserved;
    u16 pfa;
    u8  digest[SHA1_BUFSIZE];
} __attribute__((packed));


/****************************************************************
 * Physical presence interface
 ****************************************************************/

#define TPM_STATE_ENABLED 1
#define TPM_STATE_ACTIVE 2
#define TPM_STATE_OWNED 4
#define TPM_STATE_OWNERINSTALL 8

#define TPM_PPI_OP_NOOP 0
#define TPM_PPI_OP_ENABLE 1
#define TPM_PPI_OP_DISABLE 2
#define TPM_PPI_OP_ACTIVATE 3
#define TPM_PPI_OP_DEACTIVATE 4
#define TPM_PPI_OP_CLEAR 5
#define TPM_PPI_OP_SET_OWNERINSTALL_TRUE 8
#define TPM_PPI_OP_SET_OWNERINSTALL_FALSE 9

#endif // tcg.h
