.DEFAULT_GOAL := check

CARGO   ?= cargo
ARM_TARGET := thumbv7em-none-eabihf

CLIPPY_FLAGS := -D warnings
CLIPPY_PEDANTIC_FLAGS := \
	$(CLIPPY_FLAGS) \
	-W clippy::pedantic \
	-A clippy::missing_errors_doc \
	-A clippy::missing_panics_doc \
	-A clippy::must_use_candidate \
	-A clippy::return_self_not_must_use \
	-A clippy::cast_possible_truncation \
	-A clippy::cast_possible_wrap \
	-A clippy::cast_sign_loss \
	-A clippy::doc_markdown \
	-A clippy::inline_always \
	-A clippy::ptr_cast_constness \
	-A clippy::needless_for_each \
	-A clippy::borrow_as_ptr \
	-A clippy::ref_as_ptr \
	-A clippy::redundant_closure_for_method_calls \
	-A clippy::float_cmp \
	-A clippy::semicolon_if_nothing_returned \
	-A clippy::items_after_statements

DEVICE_FLAGS :=
ifdef DEVICE_NAME
DEVICE_FLAGS += --device $(DEVICE_NAME)
endif
ifdef DEVICE_ADDRESS
DEVICE_FLAGS += --address $(DEVICE_ADDRESS)
endif

.PHONY: check check-full pre-commit fmt clippy clippy-pedantic vescpkg-rs-sys-target-check arm-clippy arm-gates test package package-only deploy clean status

# --- verification -----------------------------------------------------------
#
# Policy: local `check` keeps exactly one workspace nextest invocation.
# ARM/package gates live in `pre-commit`/`check-full` so the native loopback
# binary is audited without multiplying the default test matrix.

check: fmt clippy test

check-full: check arm-gates

pre-commit: check-full

fmt:
	$(CARGO) fmt --all --check

clippy: clippy-pedantic vescpkg-rs-sys-target-check arm-clippy
	$(CARGO) clippy --workspace --all-targets --all-features -- $(CLIPPY_FLAGS)

clippy-pedantic:
	$(CARGO) clippy -p vesc-protocol -p vescpkg-rs -p vesc-example-loopback --all-targets --all-features -- $(CLIPPY_PEDANTIC_FLAGS)

vescpkg-rs-sys-target-check:
	test "$$($(CARGO) tree -p vescpkg-rs-sys --edges normal --no-default-features --prefix none | wc -l | tr -d ' ')" = 1
	$(CARGO) check -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features

arm-clippy:
	$(CARGO) clippy -p vesc-example-loopback --lib --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)

arm-gates: vescpkg-rs-sys-target-check arm-clippy package-only

test:
	$(CARGO) nextest run --workspace --features test-support

# --- packaging & device -----------------------------------------------------

package: check package-only

package-only:
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-loopback
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-alloc-smoke
	$(CARGO) run -p cargo-vescpkg -- build -p vesc-example-refloat

deploy:
	$(CARGO) run -p cargo-vescpkg -- deploy -p vesc-example-refloat $(DEVICE_FLAGS)

clean:
	$(CARGO) clean

status:
	git status --short --branch
