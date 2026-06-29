.DEFAULT_GOAL := check

CARGO   ?= cargo
PACKAGE := target/vescpkg/Rust-BLE-loopback-test-package-0.1.0/Rust-BLE-loopback-test-package-0.1.0.vescpkg

DEVICE_FLAGS :=
ifdef DEVICE_NAME
DEVICE_FLAGS += --device $(DEVICE_NAME)
endif
ifdef DEVICE_ADDRESS
DEVICE_FLAGS += --address $(DEVICE_ADDRESS)
endif

.PHONY: check check-full fmt clippy test package package-only deploy deploy-install lisp-probe clean status

# --- verification -----------------------------------------------------------
#
# Policy: local `check` runs exactly one workspace nextest invocation.
# Do NOT add extra test targets (per-crate nextest, compile-fail cargo test,
# tiered test-*, golden-check, symbol-check, etc.). Feature-matrix runs
# (cargo hack, coverage, HIL, extra feature combos) belong in CI — not here.

check: fmt clippy test

check-full: check

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p vesc-example-loopback --lib --release --target thumbv7em-none-eabihf -- -D warnings

test:
	$(CARGO) nextest run --workspace --features test-support

# --- packaging & device -----------------------------------------------------

package: check
	$(CARGO) run -p vesc-pkg --bin vesc-pkg -- package

package-only:
	$(CARGO) run -p vesc-pkg --bin vesc-pkg -- package-only

deploy: package-only
	$(CARGO) run -p vesc-cli -- deploy $(PACKAGE) $(DEVICE_FLAGS)

deploy-install: package-only
	$(CARGO) run -p vesc-cli -- package-install $(PACKAGE) $(DEVICE_FLAGS)

lisp-probe:
	$(CARGO) run -p vesc-cli -- lisp-probe $(DEVICE_FLAGS)

clean:
	$(CARGO) clean

status:
	git status --short --branch
