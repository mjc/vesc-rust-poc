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

.PHONY: check check-full pre-commit fmt clippy clippy-pedantic vescpkg-rs-sys-target-check thumb-dispatch-smoke safe-example-check arm-clippy arm-noalloc-check arm-math-check arm-alloc-check arm-alloc-math-check arm-gates test math-test alloc-math-test doc-test doc-all package package-only package-examples deploy clean status

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
	$(CARGO) clippy -p vesc-protocol -p vescpkg-rs -p vesc-example-loopback --all-targets --all-features -- $(CLIPPY_PEDANTIC_FLAGS)

vescpkg-rs-sys-target-check:
	test "$$($(CARGO) tree -p vescpkg-rs-sys --edges normal --no-default-features --prefix none | wc -l | tr -d ' ')" = 1
	$(CARGO) check -p vescpkg-rs-sys --target $(ARM_TARGET) --no-default-features

arm-noalloc-check:
	$(CARGO) check -p vescpkg-rs --target $(ARM_TARGET) --no-default-features

arm-clippy:
	$(CARGO) clippy -p vesc-example-loopback --bin vesc-example-loopback --release --target $(ARM_TARGET) -- $(CLIPPY_PEDANTIC_FLAGS)

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
