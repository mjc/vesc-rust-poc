#ifndef VESC_C_IF_H
#define VESC_C_IF_H

#include <stdbool.h>
#include <stdint.h>

typedef uint32_t lbm_value;
typedef uint32_t lbm_cid;
typedef int32_t lbm_int;
typedef unsigned int lbm_uint;
typedef uint32_t systime_t;

typedef struct lib_info {
    void (*stop_fun)(void *arg);
    void *arg;
    uint32_t base_addr;
} lib_info;

typedef lbm_value (*lbm_extension_fun)(lbm_value *args, lbm_uint argn);
typedef bool (*load_extension_fptr)(char *name, lbm_extension_fun fun);
typedef void (*app_data_handler_fun)(unsigned char *data, unsigned int len);

typedef struct {
    uint8_t *buf;
    lbm_uint buf_size;
    lbm_uint buf_pos;
} lbm_flat_value_t;

typedef struct {
    load_extension_fptr lbm_add_extension;
    void (*lbm_block_ctx_from_extension)(void);
    bool (*lbm_unblock_ctx)(lbm_cid cid, lbm_flat_value_t *value);
    lbm_cid (*lbm_get_current_cid)(void);
    int (*lbm_set_error_reason)(char *str);
    void (*lbm_pause_eval_with_gc)(uint32_t num_free);
    void (*lbm_continue_eval)(void);
    int (*lbm_send_message)(lbm_cid cid, lbm_value msg);
    bool (*lbm_eval_is_paused)(void);
    lbm_value (*lbm_cons)(lbm_value car, lbm_value cdr);
    lbm_value (*lbm_car)(lbm_value val);
    lbm_value (*lbm_cdr)(lbm_value val);
    lbm_value (*lbm_list_destructive_reverse)(lbm_value list);
    bool (*lbm_create_byte_array)(lbm_value *value, lbm_uint num_elt);
    int (*lbm_add_symbol_const)(char *name, lbm_uint *sym);
    int (*lbm_get_symbol_by_name)(char *name, lbm_uint *id);
    lbm_value (*lbm_enc_i)(lbm_int x);
    lbm_value (*lbm_enc_u)(lbm_uint x);
    lbm_value (*lbm_enc_char)(uint8_t x);
    lbm_value (*lbm_enc_float)(float f);
    lbm_value (*lbm_enc_u32)(uint32_t u);
    lbm_value (*lbm_enc_i32)(int32_t i);
    lbm_value (*lbm_enc_sym)(lbm_uint s);
    float (*lbm_dec_as_float)(lbm_value val);
    uint32_t (*lbm_dec_as_u32)(lbm_value val);
    int32_t (*lbm_dec_as_i32)(lbm_value val);
    uint8_t (*lbm_dec_char)(lbm_value x);
    char* (*lbm_dec_str)(lbm_value);
    lbm_uint (*lbm_dec_sym)(lbm_value x);
    bool (*lbm_is_byte_array)(lbm_value val);
    bool (*lbm_is_cons)(lbm_value x);
    bool (*lbm_is_number)(lbm_value x);
    bool (*lbm_is_char)(lbm_value x);
    bool (*lbm_is_symbol)(lbm_value x);
    lbm_uint lbm_enc_sym_nil;
    lbm_uint lbm_enc_sym_true;
    lbm_uint lbm_enc_sym_terror;
    lbm_uint lbm_enc_sym_eerror;
    lbm_uint lbm_enc_sym_merror;
    bool (*lbm_is_symbol_nil)(lbm_uint);
    bool (*lbm_is_symbol_true)(lbm_uint);
    uintptr_t _reserved_after_lbm[107];
    void (*send_app_data)(unsigned char *data, unsigned int len);
    bool (*set_app_data_handler)(app_data_handler_fun handler);
    uintptr_t _reserved_after_app_data[88];
    systime_t (*system_time_ticks)(void);
} vesc_c_if;

#define VESC_IF ((vesc_c_if *)(0x1000F800))
#define HEADER volatile int __attribute__((__section__(".program_ptr"))) prog_ptr;
#define INIT_FUN bool __attribute__((__section__(".init_fun"))) init
#define INIT_START (void)prog_ptr
#define INIT_END
#define ENC_SYM_EERROR ((lbm_value)0)

extern volatile int prog_ptr;

#endif
