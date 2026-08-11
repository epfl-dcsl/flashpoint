#include "../tpm_includes/tpm_driver.h"// struct tpm_driver
#include <stdbool.h>
#include "../tpm_includes/swap.h"
#include "../tpm_includes/tcg.h"
#include "../tpm_includes/riscv_io.h"
#include "../tpm_includes/tcg_tpm.h"
#include "../tpm_includes/TpmTypes.h"
#include "../tpm_includes/sha.h"

#include <stdint.h>

static TPMVersion TPM_version;

#define MAX_PCR_SELECTION_SIZE 256
static u8 tpm20_pcr_selection_storage[MAX_PCR_SELECTION_SIZE]; 

struct tpm_log_entry {
    struct tpm_log_header hdr;
    u8 pad[sizeof(struct tpm2_digest_values)
           + 8 * sizeof(struct tpm2_digest_value)
           + SHA1_BUFSIZE + SHA256_BUFSIZE + SHA384_BUFSIZE
           + SHA512_BUFSIZE + SM3_256_BUFSIZE + SHA3_256_BUFSIZE
           + SHA3_384_BUFSIZE + SHA3_512_BUFSIZE];
};
static u32 tpm20_pcr_selection_size;
static struct tpml_pcr_selection *tpm20_pcr_selection;

static const struct hash_parameters {
    u16 hashalg;
    u8  hashalg_flag;
    u8  hash_buffersize;
    const char *name;
    void (*hashfunc)(const u8 *data, u32 length, u8 *hash);
} hash_parameters[] = {
    {
        .hashalg = TPM2_ALG_SHA1,
        .hashalg_flag = TPM2_ALG_SHA1_FLAG,
        .hash_buffersize = SHA1_BUFSIZE,
        .name = "SHA1",
        .hashfunc = sha1,
    }, {
        .hashalg = TPM2_ALG_SHA256,
        .hashalg_flag = TPM2_ALG_SHA256_FLAG,
        .hash_buffersize = SHA256_BUFSIZE,
        .name = "SHA256",
        .hashfunc = sha256,
    }, {
        .hashalg = TPM2_ALG_SHA384,
        .hashalg_flag = TPM2_ALG_SHA384_FLAG,
        .hash_buffersize = SHA384_BUFSIZE,
        .name = "SHA384",
        .hashfunc = sha384,
    }
	, {
        .hashalg = TPM2_ALG_SHA512,
        .hashalg_flag = TPM2_ALG_SHA512_FLAG,
        .hash_buffersize = SHA512_BUFSIZE,
        .name = "SHA512",
        .hashfunc = sha512,
    }, {
        .hashalg = TPM2_ALG_SM3_256,
        .hashalg_flag = TPM2_ALG_SM3_256_FLAG,
        .hash_buffersize = SM3_256_BUFSIZE,
        .name = "SM3-256",
    }, {
        .hashalg = TPM2_ALG_SHA3_256,
        .hashalg_flag = TPM2_ALG_SHA3_256_FLAG,
        .hash_buffersize = SHA3_256_BUFSIZE,
        .name = "SHA3-256",
    }, {
        .hashalg = TPM2_ALG_SHA3_384,
        .hashalg_flag = TPM2_ALG_SHA3_384_FLAG,
        .hash_buffersize = SHA3_384_BUFSIZE,
        .name = "SHA3-384",
    }, {
        .hashalg = TPM2_ALG_SHA3_512,
        .hashalg_flag = TPM2_ALG_SHA3_512_FLAG,
        .hash_buffersize = SHA3_512_BUFSIZE,
        .name = "SHA3-512",
    }
};

void *memset(void *blk, int c, u64 n)
{
    u64  i;

    for (i = 0; i < n; ++i)
        ((unsigned char *) blk)[i] = c;

    return blk;
}

#define MAX_PCR_INDEX 32
static void
tpm20_set_timeouts(void)
{
	u32 durations[3] = {
		TPM2_DEFAULT_DURATION_SHORT,
		TPM2_DEFAULT_DURATION_MEDIUM,
		TPM2_DEFAULT_DURATION_LONG,
	};
	u32 timeouts[4] = {
		TIS2_DEFAULT_TIMEOUT_A,
		TIS2_DEFAULT_TIMEOUT_B,
		TIS2_DEFAULT_TIMEOUT_C,
		TIS2_DEFAULT_TIMEOUT_D,
	};

	tpmhw_set_timeouts(timeouts, durations);
}


/*
 * Helper function that constructs paylods for commands with a unique parameter.
 */
static int
tpm_simple_cmd(u8 locty, u32 ordinal
               , int param_size, u16 param, enum tpmDurationType to_t)
{
    struct {
        struct tpm_req_header trqh;
        u16 param;
    } __attribute__((packed)) req = {
        .trqh.totlen = cpu_to_be32(sizeof(req.trqh) + param_size),
        .trqh.ordinal = cpu_to_be32(ordinal),
        .param = param_size == 2 ? cpu_to_be16(param) : param,
    };
    switch (TPM_version) {
    case TPM_VERSION_1_2:
        req.trqh.tag = cpu_to_be16(TPM_TAG_RQU_CMD);
        break;
    case TPM_VERSION_2:
        req.trqh.tag = cpu_to_be16(TPM2_ST_NO_SESSIONS);
        break;
    }

    u8 obuffer[64];
    struct tpm_rsp_header *trsh = (void*)obuffer;
    u32 obuffer_len = sizeof(obuffer);
	  memset(obuffer, 0x0, sizeof(obuffer));

    int ret = tpmhw_transmit(locty, &req.trqh, obuffer, &obuffer_len, to_t);
    ret = be32_to_cpu(trsh->errcode);
    return ret;
}



/*
 * Checks capabilities (e.g. commands supported) of a TPM.
 * swtpm supports the whole array of TCG-specified commands.
 */
static int
tpm20_getcapability(u32 capability, u32 property, u32 count,
                    struct tpm_rsp_header *rsp, u32 rsize)
{
    struct tpm2_req_getcapability trg = {
        .hdr.tag = cpu_to_be16(TPM2_ST_NO_SESSIONS),
        .hdr.totlen = cpu_to_be32(sizeof(trg)),
        .hdr.ordinal = cpu_to_be32(TPM2_CC_GetCapability),
        .capability = cpu_to_be32(capability),
        .property = cpu_to_be32(property),
        .propertycount = cpu_to_be32(count),
    };

    u32 resp_size = rsize;
    int ret = tpmhw_transmit(0, &trg.hdr, rsp, &resp_size,
                             TPM_DURATION_TYPE_SHORT);
	ret = (ret ||
		   rsize < be32_to_cpu(rsp->totlen)) ? -1 : be32_to_cpu(rsp->errcode);


    return ret;
}

// static inline void *malloc_high(u32 size) {
//     return _malloc(&ZoneHigh, size, MALLOC_MIN_ALIGN);
// }

static int
tpm20_get_pcrbanks(void)
{
    u8 buffer[128];
    struct tpm2_res_getcapability *trg =
      (struct tpm2_res_getcapability *)&buffer;

    int ret = tpm20_getcapability(TPM2_CAP_PCRS, 0, 8, &trg->hdr,
                                  sizeof(buffer));
    if (ret)
        return ret;

    /* defend against (broken) TPM sending packets that are too short */
    u32 resplen = be32_to_cpu(trg->hdr.totlen);
    if (resplen <= offsetof(struct tpm2_res_getcapability, data))
        return -1;

    u32 size = resplen - offsetof(struct tpm2_res_getcapability, data);
    /* we need a valid tpml_pcr_selection up to and including sizeOfSelect */
    if (size < offsetof(struct tpml_pcr_selection, selections) +
               offsetof(struct tpms_pcr_selection, pcrSelect))
        return -1;

    if (size > MAX_PCR_SELECTION_SIZE) {
        // The TPM returned more data than our fixed buffer can hold.
        // This indicates a configuration problem or a broken TPM response.
        return -1;
    }

	tpm20_pcr_selection = (struct tpml_pcr_selection *)tpm20_pcr_selection_storage;

	// Copy the data into our pre-allocated static buffer
    memcpy(tpm20_pcr_selection, &trg->data, size);
    tpm20_pcr_selection_size = size;
 
	// tpm20_pcr_selection = malloc_high(size);
    // if (tpm20_pcr_selection) {
    //     memcpy(tpm20_pcr_selection, &trg->data, size);
    //     tpm20_pcr_selection_size = size;
    // } else {
    //     warn_noalloc();
    //     ret = -1;
    // }

    return ret;
}

/*
 * Dynamic H-CRTM operations that extends a payload into a DRTM-enabled PCR.
 */
int tpm20_drtm_operations(const u8* data, u32 len){
	u32 rc = 0;
	rc = tpm_hash_start_loc4();
	if(rc != 0){
		return -1;
	}
	timer_delay_loop(10000);
	rc = tpm_senddata_loc4(data, len);
	if(rc != 0){
		return -2;
	}
	timer_delay_loop(10000);
	rc = tpm_hash_end_loc4();
	if(rc != 0){
		return -3;
	}
	return rc;
}

/*
 * Function that reads ONLY ONE PCR. PCR indices can go up to 24.
 * Currently configured to read the SHA2-384 field of a PCR.
 */
int tpm20_read_pcrs(const u8* pcr_indices, struct read_response* resp){

	int rc = 0;

	//Ad-hoc structure that contains the necessary informations.
	//Equivalent to a TPML_PCR_SELECTION filled.
	//Everything has to be made big-endian because this informations is passthrough to libtpms and is reverted by libtpms.

	struct {
		struct tpm_req_header trqh;
		uint32_t count;
		struct tpms_pcr_selection param;
		u8 bitmap[3];
	} __attribute__((packed)) req = {
		.trqh.tag = cpu_to_be16(TPM2_ST_NO_SESSIONS),
		.trqh.totlen = 0,
		.trqh.ordinal = cpu_to_be32(TPM2_CC_PCRRead),
		.count = cpu_to_be32(1),
		.param.hashAlg = cpu_to_be16(TPM2_ALG_SHA384), 
		.param.sizeOfSelect = 3,
		.bitmap = {0}
	};
  memcpy(req.bitmap, pcr_indices, 3);
	req.trqh.totlen = cpu_to_be32(sizeof(struct tpm_req_header) +sizeof(uint32_t) + sizeof(struct tpms_pcr_selection) + 3*sizeof(u8));
	//Adapt this struct to your use case.
	

	uint32_t obuffer_len = sizeof(struct read_response);


	tpmhw_transmit(4, &req.trqh, resp, &obuffer_len, TPM_DURATION_TYPE_LONG);

	resp->trsh.tag = be16_to_cpu(resp->trsh.tag);
	resp->trsh.totlen = be32_to_cpu(resp->trsh.totlen);
	resp->pcrUpdateCounter = be32_to_cpu(resp->pcrUpdateCounter);
	resp->pcrSelectionOut.count = be32_to_cpu(resp->pcrSelectionOut.count);
	resp->pcrSels.hashAlg = be16_to_cpu(resp->pcrSels.hashAlg);
	resp->digestSize = be16_to_cpu(resp->digestSize);

	return rc;
}

/*
 * Generates a random key to be used as an access control method for keys.
 */
static int get_key_password(u8* const password){
  struct {
    struct tpm_req_header hdr;
    u16 bytes_nb;
  } __attribute__((packed)) req = {
			.hdr = {
				.tag= cpu_to_be16(TPM2_ST_NO_SESSIONS),
				.totlen = cpu_to_be32(sizeof(req)),
				.ordinal = cpu_to_be32(TPM_CC_GetRandom),
  },
    .bytes_nb = 64
  };

  struct {
    struct tpm_rsp_header rspHead;
    u16 size;
    u8 pwd[16];
  }__attribute__((packed)) rsp;

	uint32_t obuffer_len = sizeof(rsp);
	int rc = tpmhw_transmit(0, &req.hdr, &rsp, &obuffer_len, TPM_DURATION_TYPE_LONG);
  if (rc) {
    return -1;
  }
  memcpy(password, rsp.pwd, 64);
  return 0;

}


/*
 * Creates a derived object from a parent RSA key with handle `parentobject` and password `primaryPwd`
 * The key is immediately loaded into the TPM to be usable.
 */
int tpm20_createLoaded(u32 parentobject, u8* primaryPwd, u8* modulus) {
	struct {
		struct tpm2_req_createLoaded inParams;
	} __attribute__((packed)) req = {

		.inParams = {
		//Request Header
			.hdr = {
				.tag= cpu_to_be16(TPM2_ST_SESSIONS),
				.totlen = cpu_to_be32(sizeof(req)),
				.ordinal = cpu_to_be32(TPM_CC_CreateLoaded),
			},

			//TPM_RH_HIERARCHY
			.rh_hierarchy = cpu_to_be32(parentobject),
			.authblocksize = cpu_to_be32(sizeof(req.inParams.authblock)),
			.authblock = {
				.handle = cpu_to_be32(TPM2_RS_PW),
				.noncesize = cpu_to_be16(0),
				.contsession = TPM2_YES,
				.pwdsize = cpu_to_be16(64),
				.pwd = {1}
			},

      //Defines access control policy to the object as well as private data embedded in the object
			.inSensitive = {
				.size = cpu_to_be16(sizeof(req.inParams.inSensitive.sensitive)),
				.sensitive = {
					.userAuth = {
						//Dummy password value for accessing object
						.size = cpu_to_be16(64),
						.buffer = {1},
					},
					.data = {
						.size = cpu_to_be16(0),
					}
				}
			},
      //Public-accessible portion of the key parameters
			.inPublic = {
				.size = cpu_to_be16(sizeof(req.inParams.inPublic.publicArea)),
				//TPMT_PUBLIC
				.publicArea = {
          //RSA key
					.hashCategory = cpu_to_be16(TPM_ALG_RSA),
					.namingAlg = cpu_to_be16(TPM_ALG_SHA256),
					//TPMA_OBJECT
					/*//https://trustedcomputinggroup.org/wp-content/uploads/TCG_TPM2_r1p59_Part2_Structures_pub.pdf page 64*/
					/*//TPMT_PUBLIC.authPolicy (TPM2B_DIGEST)*/
					.objectAttributes.attributes = cpu_to_be32((1<<18) | (1<<16)| (1<<10)| (1<<5) | (1<<6) | (1<<1) | (1<<4)),
					.authPolicy.size = cpu_to_be16(0),
          //RSA parameters
					.rsaParms = {
						.symmetricAlg = cpu_to_be16(TPM_ALG_NULL),
						.scheme.scheme = cpu_to_be16(TPM_ALG_RSASSA),
						.scheme.details = cpu_to_be16(TPM_ALG_SHA384),
						.keyBits = cpu_to_be16((u16) 3072),
						.publicExponent = cpu_to_be32((u32) 65537),
					},
					.public_rsa_key_buffer = cpu_to_be16(0),
				//end of publicArea
				}
			//end of inPublic
			}
		//end of inParams
		}
	//end of req
	};

  //memcpy(req.inParams.authblock.pwd, primaryPwd , 64);
	struct {
		struct tpm_rsp_header trsh;
		u32 handle;
		u32 parameterSize;
		u16 privateSize;
		u8 encryptedPrivate[286];
		struct tpm2b_public_rsa outPublic; 
		u8 modulus[384];
		/*struct tpm2b_digest creationData; */
	} __attribute__((packed)) rsp;

	uint32_t obuffer_len = sizeof(rsp);
	

	tpmhw_transmit(0, &req.inParams.hdr, &rsp, &obuffer_len, TPM_DURATION_TYPE_LONG);

	if (rsp.trsh.errcode) {
		return -1; 
	}

	rsp.trsh.tag = be16_to_cpu(rsp.trsh.tag);
	rsp.trsh.totlen = be32_to_cpu(rsp.trsh.totlen);
	rsp.handle = be32_to_cpu(rsp.handle);
	memcpy(modulus, (u8*) rsp.modulus, 384);

	return rsp.handle;

}
/* We leave the choice to create primary keys under multiple seeds, i.e. under different authorities */
int tpm20_createPrimary(u32 authority, u8* password){

	struct {
		struct tpm2_req_createPrimaryRSA inParams;
	} __attribute__((packed)) req = {
		.inParams = {
			.hdr = {
				.tag= cpu_to_be16(TPM2_ST_SESSIONS),
				.totlen = cpu_to_be32(sizeof(req)),
				.ordinal = cpu_to_be32(TPM_CC_CreateLoaded),
			},
			.rh_hierarchy = cpu_to_be32(authority),
			.authblocksize = cpu_to_be32(sizeof(req.inParams.authblock)),
			.authblock = {
				.handle = cpu_to_be32(TPM2_RS_PW),
				.noncesize = cpu_to_be16(0),
				.contsession = TPM2_YES,
				.pwdsize = cpu_to_be16(0),
			},
			.inSensitive = {
				.size = cpu_to_be16(sizeof(req.inParams.inSensitive.sensitive)),
				.sensitive = {
					//TPM2B_SENSITIVE_CREATE
					//Dummy password value for accessing object
					.userAuth.size = cpu_to_be16(64),
					.userAuth.buffer = {1},
					.data.size = cpu_to_be16(64),
					.data.buffer = {1},              			
				}
			},
			.inPublic = {
				.size = cpu_to_be16(sizeof(req.inParams.inPublic.publicArea)),
				.publicArea = {
					.hashCategory = cpu_to_be16(TPM_ALG_RSA),
					.namingAlg = cpu_to_be16(TPM_ALG_SHA256),
					//https://trustedcomputinggroup.org/wp-content/uploads/TCG_TPM2_r1p59_Part2_Structures_pub.pdf page 64
					.objectAttributes.attributes = cpu_to_be32((1<<17) | (1<<16)| (1<<6)| (1<<5) | (1<<10) | (1<<1) | (1<<4)),
					//TPMT_PUBLIC.authPolicy (TPM2B_DIGEST)
					.authPolicy.size = cpu_to_be16(0),
					.rsaParms = {
						.symmetricAlg = {
							.algName = cpu_to_be16(TPM_ALG_AES),
							.keyBits = cpu_to_be16(128),
							.algMode = cpu_to_be16(TPM_ALG_CFB),
						},
						.nullAlg = cpu_to_be16(TPM_ALG_NULL),
						.keyBits = cpu_to_be16((u16) 3072),
						.publicExponent = cpu_to_be32((u32) 65537),
					},
					.public_rsa_key_buffer = cpu_to_be16(0),
				//end  of publicArea
				}
			//end of inPublic
			},
		//end of inParams
		}
	//end of req
	};

  //if (get_key_password(req.inParams.inSensitive.sensitive.data.buffer)){
  //  return -1;
  //};
  //memcpy(password, req.inParams.inSensitive.sensitive.data.buffer, 64);

	struct {
		struct tpm_rsp_header trsh;
		u32 handle;
		u16 privateSize;
		struct tpm2b_public outPublic;
		struct tpm2b_digest creationData; 
	} __attribute__((packed)) rsp;

	uint32_t obuffer_len = sizeof(rsp);
	

	tpmhw_transmit(0, &req.inParams.hdr, &rsp, &obuffer_len, TPM_DURATION_TYPE_LONG);

	rsp.trsh.errcode = be32_to_cpu(rsp.trsh.errcode);
	rsp.trsh.tag = be16_to_cpu(rsp.trsh.tag);
	rsp.handle = be32_to_cpu(rsp.handle);

	if (rsp.trsh.errcode) {
		return -1;
	}
	return rsp.handle;
}

int tpm20_quote(struct quote_verif_info* verif){
  u8 primaryPwd[64] = {0};
	u32 SRK = tpm20_createPrimary(TPM2_RH_OWNER, (u8*) primaryPwd);
	if (SRK == -1) {
		return -1;
	}
	u8 modulus[384];
	u32 aikHandle = tpm20_createLoaded(SRK, (u8*) primaryPwd, (u8*) modulus);
	if (aikHandle == -1) {
		return -1;
	}
	//printf("\n RCs: %d, %d, %d, %d \n", TPM_RC_OBJECT_MEMORY, TPM_RC_PARENT, TPM_RCS_TYPE, TPM_RCS_SIZE);
	// if (aikHandle != 0) {
	// 	if ((aikHandle >> 12) == TPM_RC_OBJECT_MEMORY) {
	// 		return 2;
	// 	} else if ((aikHandle >> 16) == TPM_RC_PARENT) {
	// 		return 3;
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_TYPE + TPM_RC_PARENT)) {
	// 		return 4;
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_SIZE + TPM_RC_SENSITIVE)) {
	// 		return 5; 
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_ATTRIBUTES)) {
	// 		return 6;
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_TYPE + TPM_RC_H + TPM_RC_1)) { 
	// 		return 7;
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_SIZE + TPM_RC_P + TPM_RC_1)) {
	// 		return 8;
	// 	} else if ((aikHandle >> 16) == (TPM_RCS_TYPE + TPM_RC_P + TPM_RC_2)) { 
	// 		return 9;
	// 	} else {
	// 		return TPM_RC_SENSITIVE;
	// 	}

	// 	//return aikHandle;
	// }
	struct {
		struct tpm2_req_quote quotePart;
		struct tpms_pcr_selection pcrSelectionIn;
		u8 bitmap[3];

	} __attribute__((packed)) req = {
		.quotePart = {
			.hdr = {
				.tag = cpu_to_be16(TPM2_ST_SESSIONS),
				.totlen = cpu_to_be32(sizeof(req)),
				.ordinal = cpu_to_be32(TPM2_CC_Quote),
			},
			.signKeyHandle = cpu_to_be32(aikHandle),
			.authblocksize = cpu_to_be32(sizeof(req.quotePart.authblock)),
			.authblock = {
				.handle = cpu_to_be32(TPM_RS_PW),
				.noncesize = cpu_to_be16(0),
				.contsession = TPM2_YES,
				.pwdsize = cpu_to_be16(64),
				.pwd = {1},
			},
			.qualifyingDatasize = cpu_to_be16(0),
			.algSchemeName = cpu_to_be16(TPM_ALG_NULL),
			.pcrSel = {
				.count = cpu_to_be32(1),
			},
		},
		.pcrSelectionIn = {
			.hashAlg = cpu_to_be16(TPM2_ALG_SHA384),
			.sizeOfSelect = 3
		},
		//PCR 0 + 17
		.bitmap = {1, 0, 2}
	};

	struct quote_response rsp;

	uint32_t obuffer_len = sizeof(rsp);

	tpmhw_transmit(0, &req.quotePart.hdr, &rsp, &obuffer_len, TPM_DURATION_TYPE_LONG);

	//timer_delay_loop(1000000);

	rsp.rspHead.hdr.tag = be16_to_cpu(rsp.rspHead.hdr.tag);
	rsp.rspHead.hdr.totlen = be32_to_cpu(rsp.rspHead.hdr.totlen);
	rsp.rspHead.hdr.errcode = be32_to_cpu(rsp.rspHead.hdr.errcode);
	if (rsp.rspHead.hdr.errcode) {
		return -3;
	}
	memcpy(verif->modulus, modulus, 384);
	memcpy(verif->signature, rsp.signature.sig.signature, 384);
	memcpy(verif->attestation, (u8*) &(rsp.rspHead.quoted.attestationData), 129);

	return 0;
}


/* In SeaBIOS, this method is wrapped into a tpm_setup method that handles linkage to the rest of the interface for the BIOS */
int tpm20_startup(void){

	//Determine which interface we're using. TIS is prefered.
	TPM_version = tpmhw_probe();
	if(tpmhw_is_present()){
		tpm20_set_timeouts();
	}
	int ret = tpm_simple_cmd(0, TPM2_CC_Startup,
							 2, TPM2_SU_CLEAR, TPM_DURATION_TYPE_SHORT);
	if (ret){
		return ret;
  	 }


    ret = tpm_simple_cmd(0, TPM2_CC_SelfTest,
                         1, TPM2_YES, TPM_DURATION_TYPE_LONG);
	if (ret) return ret;

	//u8 buffer[128];
	//struct tpm2_res_getcapability *trg =
	//  (struct tpm2_res_getcapability *)&buffer;
	//ret = tpm20_getcapability(TPM2_CAP_PCRS, 0, 8, &trg->hdr,
	// sizeof(buffer));

	ret = tpm20_get_pcrbanks();
	if (ret)
		return ret;
	return 0;
}

static void tpm2_hash_data(u16 hashAlg, const u8 *data, u32 data_len, u8 *hash)
{
    unsigned i;

    for (i = 0; i < ARRAY_SIZE(hash_parameters); i++) {
        if (hash_parameters[i].hashalg == hashAlg) {
            if (hash_parameters[i].hashfunc) {
                hash_parameters[i].hashfunc(data, data_len, hash);
            } else {
                memset(hash, 0xff, hash_parameters[i].hash_buffersize);
            }
        } 
		// else {
		// 	return -1; // Neelu: Unsupported hash alg - more algs need to be ported from seabios codebase or other sources.
		// }
    }
}

static int
tpm20_get_hash_buffersize(u16 hashAlg)
{
    unsigned i;

    for (i = 0; i < ARRAY_SIZE(hash_parameters); i++) {
        if (hash_parameters[i].hashalg == hashAlg)
            return hash_parameters[i].hash_buffersize;
    }

	return -1;
}

/*
 * Build the TPM2 tpm2_digest_values data structure from the given hash.
 * Follow the PCR bank configuration of the TPM and write the same hash
 * in either truncated or zero-padded form in the areas of all the other
 * hashes. For example, write the sha1 hash in the area of the sha256
 * hash and fill the remaining bytes with zeros. Or truncate the sha256
 * hash when writing it in the area of the sha1 hash.
 *
 * le: the log entry to build the digest in
 * hashdata: the data to hash
 * hashdata_len: the length of the hashdata
 * bigEndian: whether to build in big endian format for the TPM or
 *            little endian for the log
 *
 * Returns the digest size; -1 on fatal error
 */
static int
tpm20_build_digest(struct tpm_log_entry *le,
                   const u8 *hashdata, u32 hashdata_len, int bigEndian)
{
    if (!tpm20_pcr_selection) {
        return -15;
	}

    struct tpms_pcr_selection *sel = tpm20_pcr_selection->selections;
    void *nsel, *end = (void*)tpm20_pcr_selection + tpm20_pcr_selection_size;
    void *dest = le->hdr.digest + sizeof(struct tpm2_digest_values);

    u32 count, numAlgs = 0;
    for (count = 0; count < be32_to_cpu(tpm20_pcr_selection->count); count++) {
        u8 sizeOfSelect = sel->sizeOfSelect;

        nsel = (void*)sel + sizeof(*sel) + sizeOfSelect;
        if (nsel > end)
            break;

        /* PCR 0-7 unused? -- skip */
        if (!sizeOfSelect || sel->pcrSelect[0] == 0) {
            sel = nsel;
            continue;
        }

        int hsize = tpm20_get_hash_buffersize(be16_to_cpu(sel->hashAlg));
        if (hsize < 0) {
            // dprintf(DEBUG_tcg, "TPM is using an unsupported hash: %d\n",
            //         be16_to_cpu(sel->hashAlg));
            return -16;
        }

        /* buffer size sanity check before writing */
        struct tpm2_digest_value *v = dest;
        if (dest + sizeof(*v) + hsize > (void*)le + sizeof(*le)) {
            // dprintf(DEBUG_tcg, "tpm_log_entry is too small\n");
            return -17;
        }

        if (bigEndian)
            v->hashAlg = sel->hashAlg;
        else
            v->hashAlg = be16_to_cpu(sel->hashAlg);

        tpm2_hash_data(be16_to_cpu(sel->hashAlg), hashdata, hashdata_len,
                       v->hash);

        dest += sizeof(*v) + hsize;
        sel = nsel;

        numAlgs++;
    }

    if (sel != end) {
        // dprintf(DEBUG_tcg, "Malformed pcr selection structure fron TPM\n");
        return -18;
    }

    struct tpm2_digest_values *v = (void*)le->hdr.digest;
    if (bigEndian)
        v->count = cpu_to_be32(numAlgs);
    else
        v->count = numAlgs;

    return dest - (void*)le->hdr.digest;
}

static int tpm20_extend(struct tpm_log_entry *le, int digest_len)
{
    struct tpm2_req_extend tmp_tre = {
        .hdr.tag     = cpu_to_be16(TPM2_ST_SESSIONS),
        .hdr.totlen  = cpu_to_be32(0),
        .hdr.ordinal = cpu_to_be32(TPM2_CC_PCR_Extend),
        .pcrindex    = cpu_to_be32(le->hdr.pcrindex),
        .authblocksize = cpu_to_be32(sizeof(tmp_tre.authblock)),
        .authblock = {
            .handle = cpu_to_be32(TPM2_RS_PW),
            .noncesize = cpu_to_be16(0),
            .contsession = TPM2_YES,
            .pwdsize = cpu_to_be16(0),
        },
    };
    u8 buffer[sizeof(tmp_tre) + sizeof(le->pad)];
    struct tpm2_req_extend *tre = (struct tpm2_req_extend *)buffer;

    memcpy(tre, &tmp_tre, sizeof(tmp_tre));
    memcpy(&tre->digest[0], le->hdr.digest, digest_len);

    tre->hdr.totlen = cpu_to_be32(sizeof(tmp_tre) + digest_len);

    struct tpm_rsp_header rsp;
    u32 resp_length = sizeof(rsp);
    int ret = tpmhw_transmit(0, &tre->hdr, &rsp, &resp_length,
                             TPM_DURATION_TYPE_LONG);
    if (ret || resp_length != sizeof(rsp) || rsp.errcode) {
        //return -1;
		if (ret) { 
			return ret;
		} else if (resp_length != sizeof(rsp)) {
			return -7;
		} else if (rsp.errcode) {
			return rsp.errcode;
		}
	}

    return 0;
}

int tpm20_pcr_extend(u32 pcr_index, const u8 *hashdata, u32 hashdata_length) {
	// if (!tpm_is_working())
    //     return -1;

    struct tpm_log_entry le = {
        .hdr.pcrindex = pcr_index,
        .hdr.eventtype = EV_NO_ACTION,
    };

    int digest_len = tpm20_build_digest(&le, hashdata, hashdata_length, 1);
    if (digest_len < 0) {
        return digest_len;
	}
    int ret = tpm20_extend(&le, digest_len);
    if (ret) {
        // tpm_set_failure();
        return ret;
    }
	return 0;
}
