.DEFAULT_GOAL := check

CARGO ?= cargo

.PHONY: check test fmt clippy symbol-check package-smoke package package-only clean status

check: fmt clippy test

test:
	$(CARGO) nextest run --workspace --no-fail-fast

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --exclude vesc-rust-poc --all-targets --all-features -- -D warnings
	$(CARGO) check -p vesc-rust-poc --lib --release --target thumbv7em-none-eabihf

symbol-check:
	$(CARGO) nextest run -p vesc-pkg-build -E 'test(symbol_audit)'

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
