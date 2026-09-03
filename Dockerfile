# Kinetix pricing service — Rust / Rocket / Diesel-async / tonic.
#
# Two stages. The builder carries the Rust toolchain and libpq headers; the runtime carries
# neither, only the shared library Diesel links against. protoc is not installed: build.rs
# uses `protoc-bin-vendored`, so the compiler ships inside the crate and the build does not
# depend on a system protobuf that differs between the CI runner and this image.
#
# Rust 1.88, not 1.83: Cargo.lock resolves `time-core 0.1.9`, which declares edition2024.
# That feature is not stabilised before 1.85, so an older toolchain cannot even parse the
# manifest — the build fails at dependency download, before compiling a line of this crate.

FROM rust:1.88-slim-bookworm@sha256:38bc5a86d998772d4aec2348656ed21438d20fcdce2795b56ca434cf21430d89 AS build

RUN apt-get update && apt-get install --no-install-recommends -y \
        libpq-dev pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# Dependency layer first, so a source-only change does not rebuild every crate. The dummy
# targets exist purely to give cargo something to compile against the real manifest.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked 2>/dev/null || true \
    && rm -rf src

COPY build.rs ./
COPY proto ./proto
COPY src ./src
COPY migrations ./migrations
COPY Rocket.toml ./

# Touch the real sources so cargo does not reuse the dummy fingerprints above.
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS final

# curl is here for the compose healthcheck, which probes /health/ready over HTTP. Without it
# this image has no way at all to make an HTTP request, and the service could only be checked
# by "is the process alive" — which is exactly how it ran for its whole life against a database
# with no tables.
RUN apt-get update && apt-get install --no-install-recommends -y \
        libpq5 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 pricing \
    && useradd --system --uid 10001 --gid 10001 --no-create-home pricing

WORKDIR /app

COPY --from=build /src/target/release/kinetix-pricing-service /usr/local/bin/kinetix-pricing-service
COPY --from=build --chown=10001:10001 /src/migrations /app/migrations
COPY --from=build --chown=10001:10001 /src/Rocket.toml /app/Rocket.toml

USER 10001:10001

# REST on 6000, gRPC on 50054. Neither is published to the host — the gateway reaches this
# over the `kinetix` network by service name.
EXPOSE 6000 50054

ENTRYPOINT ["kinetix-pricing-service"]
