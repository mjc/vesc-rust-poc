#ifndef VESC_C_IF_H
#define VESC_C_IF_H

#include <stdint.h>

typedef uintptr_t lbm_value;
typedef unsigned int lbm_uint;

typedef struct lib_info {
    const char *name;
    const char *version;
} lib_info;

typedef lbm_value (*lbm_extension_fun)(lbm_value *args, lbm_uint argn);

#define INIT_FUN(name) void name(lib_info *info)
#define INIT_START
#define INIT_END
#define ENC_SYM_EERROR ((lbm_value)0)

int lbm_add_extension(const char *name, lbm_extension_fun fun);
int32_t lbm_dec_as_i32(lbm_value value);
lbm_value lbm_enc_i(int32_t value);

#endif
