FROM rust:1.75-bookworm

COPY ./bin/bme680-exporter_aarch64-unknown-linux-gnu bme680-exporter

CMD ["/bme680-exporter"]
