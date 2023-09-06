FROM rust:slim
WORKDIR /app

COPY target/release/mystiko_relayer /app

EXPOSE 8090
CMD ["./mystiko_relayer", "config.toml"]
