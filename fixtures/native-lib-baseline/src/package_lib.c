#include "vesc_c_if.h"

lbm_value ext_c_probe_v6(lbm_value *args, lbm_uint argn) {
    (void)args;
    (void)argn;

    return VESC_IF->lbm_enc_i(42);
}
