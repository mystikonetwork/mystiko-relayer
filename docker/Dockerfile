FROM rust:slim
WORKDIR /app

RUN apt update && \
    apt install -y curl

COPY target/release/mystiko_relayer /app

EXPOSE 8090
CMD ["./mystiko_relayer", "config.toml"]
