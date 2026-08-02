.DEFAULT_GOAL := check

CARGO   ?= cargo
ARM_TARGET := thumbv7em-none-eabihf

CLIPPY_FLAGS := -D warnings
CLIPPY_PEDANTIC_FLAGS := \
	$(CLIPPY_FLAGS) \
	-W clippy::pedantic

DEVICE_FLAGS :=
ifdef DEVICE_NAME
DEVICE_FLAGS += --device $(DEVICE_NAME)
endif
ifdef DEVICE_ADDRESS
DEVICE_FLAGS += --address $(DEVICE_ADDRESS)
endif

.PHONY: check check-full pre-commit fmt clippy clippy-pedantic vescpkg-rs-sys-target-check thumb-dispatch-smoke safe-example-check arm-clippy arm-noalloc-check arm-math-check arm-alloc-check arm-alloc-math-check arm-gates test math-test alloc-math-test doc-test doc-all fob-production-loc package package-only package-examples deploy clean status

# --- verification -----------------------------------------------------------
#
# Policy: local `check` keeps exactly one workspace nextest invocation.
# ARM/package gates live in `pre-commit`/`check-full` so the native loopback
# binary is audited without multiplying the default test matrix.

check: fmt clippy test doc-test

check-full: check arm-gates safe-example-check math-test alloc-math-test doc-all

pre-commit: check-full

fmt:
	$(CARGO) fmt --all --check

clippy: clippy-pedantic vescpkg-rs-sys-target-check arm-clippy
	$(CARGO) clippy --workspace --all-targets --all-features -- $(CLIPPY_FLAGS)

clippy-pedantic:
	$(CARGO) clippy \
		-p vesc-protocol \
		-p vescpkg-rs-sys \
		-p vescpkg-rs-units \
		-p vescpkg-rs \
		-p vesc-example-alloc-smoke \
		-p vesc-example-loopback \
		-p vesc-example-float-out-boy \
		--all-targets \
		--all-features \
		-- $(CLIPPY_PEDANTIC_FLAGS)
	# `--all-features` enables `alloc`, so also check the supported host test
	# helpers without it; conditional imports can otherwise escape this gate.
	$(CARGO) clippy -p vescpkg-rs --all-targets --no-default-features --features test-support -- $(CLIPPY_PEDANTIC_FLAGS)

vescpkg-rs-sys-target-check:
	test "$$($(CARGO) tree -p vescpkg-rs-sys --edges normal --no-default-features --prefix none | wc -l | tr -d ' ')" = 1
	$(CARGO) check -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features

arm-noalloc-check:
	$(CARGO) check -p vescpkg-rs --target $(ARM_TARGET) --no-default-features

arm-clippy:
	$(CARGO) clippy -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-loopback --bin vesc-example-loopback --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-alloc-smoke --bin vesc-example-alloc-smoke --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-control-loop-smoke --bin vesc-example-control-loop-smoke --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-float-out-boy --bin vesc-example-float-out-boy --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)

arm-math-check:
	$(CARGO) check -p vescpkg-rs --target $(ARM_TARGET) --no-default-features --features math

arm-alloc-check:
	$(CARGO) check -p vescpkg-rs --target $(ARM_TARGET) --no-default-features --features alloc

arm-alloc-math-check:
	$(CARGO) check -p vescpkg-rs --target $(ARM_TARGET) --no-default-features --features 'alloc math'

arm-gates: vescpkg-rs-sys-target-check thumb-dispatch-smoke arm-noalloc-check arm-clippy arm-math-check arm-alloc-check arm-alloc-math-check package-examples

thumb-dispatch-smoke:
	./tools/thumb-dispatch-smoke.sh

safe-example-check:
	./tools/safe-example-check.sh

test:
	$(CARGO) nextest run --workspace --features test-support

math-test:
	$(CARGO) nextest run -p vescpkg-rs --features 'test-support math'

alloc-math-test:
	$(CARGO) nextest run -p vescpkg-rs --features 'test-support alloc math'

doc-test:
	$(CARGO) test --doc --workspace

doc-all:
	$(CARGO) doc --workspace --all-features --no-deps

fob-production-loc:
	./tools/fob-production-tokei.sh

# --- packaging & device -----------------------------------------------------

package: check package-only

package-only:
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-float-out-boy

# Build the representative package set used by the package proof.
package-examples:
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-loopback
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-alloc-smoke
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-control-loop-smoke
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-float-out-boy

deploy:
	$(CARGO) run -p cargo-vescpkg -- deploy -p vesc-example-loopback $(DEVICE_FLAGS)

clean:
	$(CARGO) clean

status:
	git status --short --branch
