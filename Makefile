.DEFAULT_GOAL := check

CARGO ?= cargo

.PHONY: check test test-all test-changed fmt clippy symbol-check package-smoke package package-only clean status coverage coverage-ffi coverage-package

check: fmt clippy test

test: test-all

test-all:
	$(CARGO) nextest run --workspace --no-fail-fast --features test-support

test-changed:
	$(CARGO) test-changed -r nextest

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p vesc-ble-loopback --lib --release --target thumbv7em-none-eabihf -- -D warnings

symbol-check:
	$(CARGO) nextest run -p vesc-pkg-build -E 'test(symbol_audit)'

COVERAGE_FAIL_UNDER ?= 80
COVERAGE_PACKAGE_IGNORE := --ignore-filename-regex 'crates/vesc-(ffi|protocol)/'

coverage-ffi:
	$(CARGO) llvm-cov -p vesc-ffi --features test-support --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-package:
	$(CARGO) llvm-cov -p vesc-package --features test-support $(COVERAGE_PACKAGE_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage: coverage-ffi coverage-package

package-smoke:
	$(CARGO) nextest run -p vesc-pkg-build package_payload_stays_well_below_the_vesc_tool_flash_block_limit

package: check
	$(CARGO) run -p vesc-pkg-build --bin vesc-pkg -- package

package-only:
	$(CARGO) run -p vesc-pkg-build --bin vesc-pkg -- package-only

clean:
	$(CARGO) clean

status:
	git status --short --branch
