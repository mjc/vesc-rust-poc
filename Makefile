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

.PHONY: check check-full pre-commit fmt clippy clippy-pedantic vescpkg-rs-sys-target-check arm-clippy arm-gates test doc-test package package-only deploy clean status

# --- verification -----------------------------------------------------------
#
# Policy: local `check` keeps exactly one workspace nextest invocation.
# ARM/package gates live in `pre-commit`/`check-full` so the native loopback
# binary is audited without multiplying the default test matrix.

check: fmt clippy test doc-test

check-full: check arm-gates

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

vescpkg-rs-sys-target-check:
	test "$$($(CARGO) tree -p vescpkg-rs-sys --edges normal --no-default-features --prefix none | wc -l | tr -d ' ')" = 1
	$(CARGO) check -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features

arm-clippy:
	$(CARGO) clippy -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-loopback --bin vesc-example-loopback --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)
	$(CARGO) clippy -p vesc-example-float-out-boy --bin vesc-example-float-out-boy --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)

arm-gates: vescpkg-rs-sys-target-check arm-clippy package-only

test:
	$(CARGO) nextest run --workspace --features test-support

doc-test:
	$(CARGO) test --doc --workspace

# --- packaging & device -----------------------------------------------------

package: check package-only

package-only:
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-float-out-boy

deploy:
	$(CARGO) run -p cargo-vescpkg -- deploy -p vesc-example-loopback $(DEVICE_FLAGS)

clean:
	$(CARGO) clean

status:
	git status --short --branch
