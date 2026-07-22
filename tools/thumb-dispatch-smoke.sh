#!/usr/bin/env bash
set -euo pipefail

target=thumbv7em-none-eabihf
target_dir=$(mktemp -d)
trap 'rm -rf "$target_dir"' EXIT

CARGO_TARGET_DIR="$target_dir" cargo rustc \
  -p vescpkg-rs-sys \
  --target "$target" \
  --no-default-features \
  --lib \
  --release \
  -- \
  --emit=obj \
  -C embed-bitcode=yes \
  -C link-dead-code=yes

bitcode=$(find "$target_dir/$target/release/deps" -name 'vescpkg_rs_sys-*.o' -print -quit)
test -n "$bitcode"
llvm_dir="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin"
object="$target_dir/vescpkg-rs-sys.o"
disassembly="$target_dir/vescpkg-rs-sys.asm"
"$llvm_dir/llc" -mtriple="$target" -filetype=obj "$bitcode" -o "$object"
"$llvm_dir/llvm-objdump" -d --symbol-description "$object" > "$disassembly"

manifest=$(find "$target_dir/$target/release/build" -path '*/out/c_vesc_if.rs' -print -quit)
test -n "$manifest"

slot_index() {
    awk -v module="pub(crate) mod $1 {" '
        $0 == module { in_module = 1; next }
        in_module && /pub\(crate\) const INDEX:/ {
            gsub(/[^0-9]/, "", $0); print; exit
        }
        in_module && /^}/ { exit }
    ' "$manifest"
}

assert_load() {
    local symbol=$1
    local slot=$2
    local offset
    offset=$(printf '0x%x' "$((slot * 4))")
    sed -n "/<.*${symbol}>:/,/^$/p" "$disassembly" | rg -q "ldr\\.w.*#${offset}"
}

assert_load sleep_us "$(slot_index sleep_us)"
assert_load can_transmit_sid "$(slot_index can_transmit_sid)"
assert_load vesc_system_time_ticks "$(slot_index system_time_ticks)"
assert_load vesc_thread_set_priority "$(slot_index thread_set_priority)"

system_time_ticks=$(sed -n '/<.*vesc_system_time_ticks>:/,/^$/p' "$disassembly")
printf '%s' "$system_time_ticks" | rg -q 'cbz'
printf '%s' "$system_time_ticks" | rg -q 'blx'
