.DEFAULT_GOAL := check

CARGO ?= cargo

.PHONY: check check-full test test-all test-embedded test-package test-changed fmt clippy symbol-check package-smoke package package-only clean status coverage coverage-ffi coverage-package coverage-pkg-build coverage-host-cli

check: fmt clippy test

check-full: check symbol-check test-package

test: test-all

test-all:
	$(CARGO) nextest run --workspace --no-fail-fast --features test-support --profile default

test-embedded:
	$(CARGO) nextest run -p vesc-pkg-build --no-fail-fast --profile embedded

test-package:
	$(CARGO) nextest run -p vesc-pkg-build --no-fail-fast --features test-support --profile package

test-changed:
	$(CARGO) test-changed -r nextest

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p vesc-ble-loopback --lib --release --target thumbv7em-none-eabihf -- -D warnings

symbol-check: test-embedded

COVERAGE_FAIL_UNDER ?= 80
COVERAGE_PACKAGE_IGNORE := --ignore-filename-regex 'crates/vesc-(ffi|protocol)/'
COVERAGE_PKG_BUILD_IGNORE := --ignore-filename-regex 'crates/vesc-pkg-build/tests/|test_support'
COVERAGE_HOST_CLI_IGNORE := --ignore-filename-regex 'tests/fake_ble_integration'

coverage-ffi:
	$(CARGO) llvm-cov -p vesc-ffi --features test-support --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-package:
	$(CARGO) llvm-cov -p vesc-package --features test-support $(COVERAGE_PACKAGE_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-pkg-build:
	$(CARGO) llvm-cov -p vesc-pkg-build --features test-support $(COVERAGE_PKG_BUILD_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage-host-cli:
	$(CARGO) llvm-cov -p vesc-host-cli $(COVERAGE_HOST_CLI_IGNORE) --summary-only --fail-under-lines $(COVERAGE_FAIL_UNDER)

coverage: coverage-ffi coverage-package coverage-pkg-build coverage-host-cli

package-smoke: test-package

package: check
	$(CARGO) run -p vesc-pkg-build --bin vesc-pkg -- package

package-only:
	$(CARGO) run -p vesc-pkg-build --bin vesc-pkg -- package-only

clean:
	$(CARGO) clean

status:
	git status --short --branch
