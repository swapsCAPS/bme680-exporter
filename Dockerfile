FROM rust:1.40-stretch
COPY ./target/release/bme680-exporter .
EXPOSE 4242
CMD ["/bme680-exporter"]
