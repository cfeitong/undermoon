build:
	cargo build

release:
	cargo build --release

server:
	RUST_LOG=undermoon=debug,server_proxy=debug target/debug/server_proxy

coord:
	RUST_LOG=undermoon=debug,coordinator=debug target/debug/coordinator

flamegraph:
	sudo flamegraph -o my_flamegraph.svg target/release/server_proxy

syscall:
	scripts/syscall.sh

.PHONY: build server coord
