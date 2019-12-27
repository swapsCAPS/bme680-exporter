build_release:
	@cargo build --target=armv7-unknown-linux-gnueabihf --release
	@cp ./target/armv7-unknown-linux-gnueabihf/release/bme680-exporter .

build_docker:
	@docker build . -t bme680-exporter
