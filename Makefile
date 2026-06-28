.DEFAULT_GOAL := check

CARGO ?= cargo

.PHONY: check check-full check-ffi check-ffi-header test test-all test-embedded test-ffi test-package test-changed fmt clippy symbol-check golden-check package-smoke package package-only deploy deploy-install clean status coverage coverage-ffi coverage-sdk coverage-pkg coverage-cli hack-check

check: fmt clippy test

check-full: check check-ffi-header symbol-check test-package

hack-check:
	$(CARGO) hack check --each-feature -p vesc-sdk
	$(CARGO) hack check --each-feature -p vesc-pkg
	$(CARGO) hack check --each-feature -p vesc-example-loopback --lib --release --target thumbv7em-none-eabihf

test: test-all

test-all:
	$(CARGO) nextest run --workspace --no-fail-fast --features test-support --profile default
	$(CARGO) test -p vesc-ffi compile_fail_ui --no-default-features

test-ffi:
	$(CARGO) nextest run -p vesc-ffi -p vesc-pkg --lib --features test-support --profile ffi

check-ffi-header:
	$(CARGO) test -p vesc-pkg --lib ffi_compare

check-ffi: test-ffi coverage-ffi check-ffi-header

test-embedded:
	$(CARGO) nextest run -p vesc-pkg --no-fail-fast --profile embedded

test-package:
	$(CARGO) nextest run -p vesc-pkg --no-fail-fast --features test-support --profile package

test-changed:
	$(CARGO) test-changed -r nextest

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p vesc-example-loopback --lib --release --target thumbv7em-none-eabihf -- -D warnings

symbol-check: test-embedded

golden-check:
	$(CARGO) test -p vesc-pkg package_golden -- --test-threads=1

COVERAGE_FAIL_UNDER ?= 80
COVERAGE_PACKAGE_IGNORE := --ignore-filename-regex 'crates/vesc-(ffi|protocol)/'
COVERAGE_PKG_IGNORE := --ignore-filename-regex 'crates/vesc-pkg/tests/|test_support'
COVERAGE_CLI_IGNORE := --ignore-filename-regex 'tests/fake_ble_integration'

coverage-ffi:
	$(CARGO) llvm-cov -p vesc-ffi --lib --features test-support --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER) -- --test-threads=1

coverage-sdk:
	$(CARGO) llvm-cov -p vesc-sdk --features test-support $(COVERAGE_PACKAGE_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-pkg:
	$(CARGO) llvm-cov -p vesc-pkg --features test-support $(COVERAGE_PKG_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-cli:
	$(CARGO) llvm-cov -p vesc-cli $(COVERAGE_CLI_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage: coverage-ffi coverage-sdk coverage-pkg coverage-cli

package-smoke: test-package

package: check
	$(CARGO) run -p vesc-pkg --bin vesc-pkg -- package

package-only:
	$(CARGO) run -p vesc-pkg --bin vesc-pkg -- package-only

PACKAGE_ARTIFACT := target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg
DEVICE_FLAGS ?=
ifdef DEVICE_NAME
DEVICE_FLAGS += --device $(DEVICE_NAME)
endif
ifdef DEVICE_ADDRESS
DEVICE_FLAGS += --address $(DEVICE_ADDRESS)
endif

deploy: package
	$(CARGO) run -p vesc-cli -- package-install $(PACKAGE_ARTIFACT) $(DEVICE_FLAGS)
	$(CARGO) run -p vesc-cli -- loopback $(DEVICE_FLAGS)

deploy-install: package
	$(CARGO) run -p vesc-cli -- package-install $(PACKAGE_ARTIFACT) $(DEVICE_FLAGS)

clean:
	$(CARGO) clean

status:
	git status --short --branch
