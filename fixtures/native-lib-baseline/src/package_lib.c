#include <stdint.h>

#include "vesc_c_if.h"

extern void package_lib_init(lib_info *info);

HEADER

int lbm_add_extension(const char *name, lbm_extension_fun fun) {
    return VESC_IF->lbm_add_extension((char *)name, fun);
}

int32_t lbm_dec_as_i32(lbm_value value) {
    return VESC_IF->lbm_dec_as_i32(value);
}

lbm_value lbm_enc_i(int32_t value) {
    return VESC_IF->lbm_enc_i(value);
}

INIT_FUN {
    INIT_START;
    package_lib_init(0);
    return true;
}
