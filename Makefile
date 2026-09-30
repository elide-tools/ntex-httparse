.PHONY: test bench fmt

test:
	cargo test $(ARGS)

bench:
	cargo test --release simd::tests::benchmark_backends -- --ignored --nocapture

fmt:
	cargo fmt
