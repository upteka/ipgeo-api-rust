FROM rust:slim as builder

# 安装必要的构建工具
RUN apt-get update && apt-get install -y pkg-config libssl-dev musl-tools && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/ipgeo

# 添加 musl 目标
RUN rustup target add x86_64-unknown-linux-musl

# 设置 cargo 配置以优化构建速度
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true \
    CARGO_BUILD_JOBS=8 \
    RUSTC_FLAGS="-C target-cpu=native -C opt-level=3" \
    CARGO_INCREMENTAL=1 \
    RUST_BACKTRACE=1 \
    RUST_LOG=info

# 创建缓存层
RUN cargo init
COPY Cargo.toml Cargo.lock* ./

# 预编译依赖
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src/

# 编译实际代码
COPY src src/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cp target/x86_64-unknown-linux-musl/release/ipgeo /usr/local/bin/ && \
    strip /usr/local/bin/ipgeo

FROM alpine:3.19

# 只安装必要的运行时依赖
RUN apk add --no-cache curl ca-certificates

WORKDIR /app

# 创建数据目录
RUN mkdir -p /app/data

COPY --from=builder /usr/local/bin/ipgeo ./ipgeo
COPY update_mmdb.sh .
RUN chmod +x update_mmdb.sh && chmod +x ipgeo

# 设置运行时环境变量
ENV RUST_BACKTRACE=1 \
    RUST_LOG=info \
    MMDB_PATH=/app/data

EXPOSE 8080

# 使用非 root 用户运行
RUN adduser -D -h /app appuser && \
    chown -R appuser:appuser /app
USER appuser

# 声明数据卷
VOLUME ["/app/data"]

# 先运行更新脚本，然后启动服务器
ENTRYPOINT ["sh", "-c", "./update_mmdb.sh && ./ipgeo"] 