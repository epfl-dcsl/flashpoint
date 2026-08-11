#ifndef TCGBIOS_H
#define TCGBIOS_H

#include "typedefs.h"
#include "TpmTypes.h"
#include "tcg.h"

#define TPM_VERSION 0

struct quote_response{
	struct tpm2_quote_rsp rspHead;
	u8 bitmap[3];
	u16 digestSize;
	u8 digest[SHA384_BUFSIZE];
	struct tpmt_signature_rsa signature;
	u8 buffer[5];

} __attribute__((packed));


struct read_response{
  struct tpm_rsp_header trsh;
  uint32_t pcrUpdateCounter;
  struct tpml_pcr_selection pcrSelectionOut;
  struct tpms_pcr_selection pcrSels;
  u8 bitmap[3];
  struct tpm2_digest_values pcrValues;
  u16 digestSize;
  u8 digest[SHA384_BUFSIZE]; 
} __attribute__((packed));

struct quote_verif_info{
	u8 modulus[384];
	u8 signature[384];
	u8 attestation[384];
} __attribute((packed));

void tpm_setup(void);
void tpm_prepboot(void);
void tpm_s3_resume(void);
void tpm_add_bcv(u32 bootdrv, const u8 *addr, u32 length);
void tpm_add_cdrom(u32 bootdrv, const u8 *addr, u32 length);
void tpm_add_cdrom_catalog(const u8 *addr, u32 length);
void tpm_option_rom(const void *addr, u32 len);
int tpm_can_show_menu(void);
void tpm_menu(void);
int tpm20_startup(void);
int tpm20_drtm_operations(const u8* data, u32 len);
int tpm20_read_pcrs(const u8* pcr_indices, struct read_response* resp);
int tpm20_quote(struct quote_verif_info* rsp);
int tpm20_pcr_extend(u32 pcr_index, const u8 *hashdata, u32 hashdata_length);

#endif /* TCGBIOS_H */
