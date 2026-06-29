__asm__(".section .note.GNU-stack,\"\",%progbits");

#include "vesc_c_if.h"

extern void loopback_handle_app_data(unsigned char *data, unsigned int len);

static void loopback_app_data_rx(unsigned char *data, unsigned int len) {
	if (!data || len == 0) {
		return;
	}

	loopback_handle_app_data(data, len);
}

bool vesc_register_loopback_app_data_handler(void) {
	return VESC_IF->set_app_data_handler(loopback_app_data_rx);
}

void vesc_clear_loopback_app_data_handler(void) {
	VESC_IF->set_app_data_handler(0);
}
