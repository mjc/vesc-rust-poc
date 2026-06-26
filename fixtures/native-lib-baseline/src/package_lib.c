#include "vesc_c_if.h"

extern void package_lib_init(lib_info *info);

HEADER

INIT_FUN(lib_info *info) {
    INIT_START;
    package_lib_init(info);
    return true;
}
