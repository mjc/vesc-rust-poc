#include <stdint.h>

#include "vesc_c_if.h"

extern int32_t rust_add(int32_t a, int32_t b);

static lbm_value ext_rust_add(lbm_value *args, lbm_uint argn) {
    if (argn != 2) {
        return ENC_SYM_EERROR;
    }

    int32_t a = lbm_dec_as_i32(args[0]);
    int32_t b = lbm_dec_as_i32(args[1]);

    return lbm_enc_i(rust_add(a, b));
}

INIT_FUN(package_lib_init) {
    INIT_START;
    lbm_add_extension("ext-rust-add", ext_rust_add);
    INIT_END;
}
