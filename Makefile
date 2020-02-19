build:
	cargo build

test:
	RUST_BACKTRACE=full cargo test -- --nocapture

lint:
	find src -name "*.rs" | xargs rustup run stable rustfmt
	cargo clippy

release:
	cargo build --release

server:
	RUST_LOG=undermoon=debug,server_proxy=debug target/debug/server_proxy conf/server-proxy.toml

server-release:
	RUST_LOG=undermoon=info,server_proxy=info target/release/server_proxy conf/server-proxy.toml

coord:
	RUST_LOG=undermoon=debug,coordinator=debug target/debug/coordinator conf/coordinator.toml

broker:
	RUST_LOG=undermoon=debug,mem_broker=debug target/debug/mem_broker conf/mem-broker.toml

flame:
	sudo flamegraph -o $(name).svg target/release/server_proxy conf/server-proxy.toml

docker-build-image:
	docker image build -f examples/Dockerfile-builder -t undermoon_builder .
	sh scripts/dkrebuild.sh
	docker image build -f examples/Dockerfile-undermoon -t undermoon .

docker-rebuild-bin:
	sh scripts/dkrebuild.sh

docker-build-release:
	docker image build -f examples/Dockerfile-builder-release -t undermoon_builder_release .
	mkdir -p ./examples/target_volume/release
	docker rm undermoon-builder-container || true
	docker create -it --name undermoon-builder-container undermoon_builder_release bash
	docker cp undermoon-builder-container:/undermoon/target/release/server_proxy ./examples/target_volume/release/
	docker cp undermoon-builder-container:/undermoon/target/release/coordinator ./examples/target_volume/release/
	docker cp undermoon-builder-container:/undermoon/target/release/mem_broker ./examples/target_volume/release/
	docker rm undermoon-builder-container
	docker image build -f examples/Dockerfile-undermoon-release -t undermoon .

docker-multi-redis:
	docker-compose -f examples/docker-compose-multi-redis.yml up

docker-multi-shard:
	docker-compose -f examples/docker-compose-multi-shard.yml up

docker-failover:
	docker-compose -f examples/docker-compose-multi-shard.yml -f examples/docker-compose-failover.yml up

docker-mem-broker:
	docker-compose -f examples/docker-compose-mem-broker.yml up

docker-overmoon:
	# Need to build the 'overmoon' image first
	# > git clone https://github.com/doyoubi/overmoon
	# > cd overmoon
	# > make build-docker
	docker-compose -f examples/docker-compose-overmoon.yml up

.PHONY: build test lint release server coord test_broker flame docker-build-image docker-multi-redis docker-multi-shard docker-failover docker-mem-broker docker-overmoon

