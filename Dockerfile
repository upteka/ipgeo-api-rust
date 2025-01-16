FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /app

# 安装必要的构建依赖
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig

FROM chef AS planner
COPY Cargo.* .
COPY src src/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# 安装必要的构建依赖
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig

COPY --from=planner /app/recipe.json recipe.json

# 构建依赖
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# 现在复制源代码并构建
COPY . .
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_BUILD_JOBS=16 \
    RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C codegen-units=1 -C debug-assertions=no" \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    CARGO_PROFILE_RELEASE_PANIC="abort" \
    CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=false \
    CARGO_PROFILE_RELEASE_INCREMENTAL=false \
    RUST_BACKTRACE=1 \
    RUST_LOG=info \
    OPENSSL_STATIC=1 \
    PKG_CONFIG_ALL_STATIC=1

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/ipgeo /usr/local/bin/ && \
    strip /usr/local/bin/ipgeo

FROM alpine:3.19

# 只安装必要的运行时依赖
RUN apk add --no-cache ca-certificates file

COPY --from=builder /usr/local/bin/ipgeo /usr/local/bin/
COPY data /app/data/

WORKDIR /app
ENV RUST_LOG=info

EXPOSE 3000
CMD ["ipgeo"] 