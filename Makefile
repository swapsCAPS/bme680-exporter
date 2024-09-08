build_release:
	@cargo build --target=armv7-unknown-linux-gnueabihf --release
	@cp ./target/armv7-unknown-linux-gnueabihf/release/bme680-exporter .

build-pi:
	cross build --release --target aarch64-unknown-linux-gnu
	cp target/aarch64-unknown-linux-gnu/release/bme680-exporter bin/bme680-exporter_aarch64-unknown-linux-gnu

build_docker:
	@docker build . -t bme680-exporter
