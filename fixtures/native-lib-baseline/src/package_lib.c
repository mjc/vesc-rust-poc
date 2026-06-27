#include "vesc_c_if.h"

HEADER

extern bool package_lib_init(lib_info *info);

static lbm_value ext_c_probe_v12(lbm_value *args, lbm_uint argn) {
    if (argn != 1 || !VESC_IF->lbm_is_number(args[0])) {
        return VESC_IF->lbm_enc_sym_eerror;
    }

    int32_t v = VESC_IF->lbm_dec_as_i32(args[0]) * 3;
    return ((lbm_value)v << 4) | 8u;
}

INIT_FUN(lib_info *info) {
    INIT_START;

    (void)package_lib_init(info);
    VESC_IF->lbm_add_extension("ext-c-probe-v12", ext_c_probe_v12);
    return true;
}

__asm__(".section .note.GNU-stack,\"\",%progbits");
