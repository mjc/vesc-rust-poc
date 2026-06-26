#include "vesc_c_if.h"

extern void package_lib_init(lib_info *info);

HEADER

static lbm_value ext_c_probe_v6(lbm_value *args, lbm_uint argn) {
    (void)args;
    (void)argn;

    return VESC_IF->lbm_enc_i(42);
}

INIT_FUN(lib_info *info) {
    INIT_START;
    package_lib_init(info);
    VESC_IF->lbm_add_extension("ext-c-probe-v6", ext_c_probe_v6);
    return true;
}
