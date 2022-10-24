FROM rust:slim-bullseye AS buildstage
WORKDIR /build
ENV PROTOC_NO_VENDOR 1
RUN rustup component add rustfmt && \
    apt-get update && \
    apt-get install -y --no-install-recommends wget && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
COPY . /build/
RUN cargo build --release

FROM debian:bullseye-slim
RUN useradd -m chain
USER chain
COPY --from=buildstage /build/target/release/client /usr/bin/
CMD ["client"]
