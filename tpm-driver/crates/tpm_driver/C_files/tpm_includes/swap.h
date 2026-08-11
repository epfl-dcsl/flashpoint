
#include "typedefs.h"

static inline u32 be32_to_cpu(u32 val){
	return (val<<24) | ((val&0xff00) << 8) | ((val & 0xff0000) >> 8) | (val>>24);
}

static inline u16 be16_to_cpu(u16 val){
	return (val << 8) | (val >> 8);
}

static inline u64 be64_to_cpu(u64 val){
	return(val<<56) | ((val & 0xff00) << 40) | ((val & 0xff0000) << 24)| ((val & 0xff000000) << 8) | ((val & 0xff00000000) >> 8) | 
											   ((val & 0xff0000000000) >> 24) | ((val & 0xff000000000000) >> 40) | (val >> 56) ;
}

#define cpu_to_be32(x) be32_to_cpu(x)
#define cpu_to_be16(x) be16_to_cpu(x)
#define cpu_to_be64(x) be64_to_cpu(x)
