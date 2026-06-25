.DEFAULT_GOAL := check

CARGO ?= cargo

.PHONY: check test fmt clippy symbol-check package-smoke clean status

check: test fmt clippy symbol-check package-smoke

test:
	$(CARGO) test --workspace

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --exclude vesc-rust-poc --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p vesc-rust-poc --lib --release --target thumbv7em-none-eabihf -- -D warnings

symbol-check:
	$(CARGO) test -p vesc-pkg-build symbol_audit

package-smoke:
	$(CARGO) test -p vesc-pkg-build package_payload_stays_well_below_the_vesc_tool_flash_block_limit

clean:
	$(CARGO) clean

status:
	git status --short --branch
