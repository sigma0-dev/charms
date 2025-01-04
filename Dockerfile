FROM rust:alpine AS base
RUN apk add --no-cache musl-dev openssl-dev pkgconfig openssl-libs-static

FROM base AS builder
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,from=base,source=/usr/local/rustup,target=/usr/local/rustup \
    cargo install --locked --path .

FROM alpine AS runtime
COPY --from=builder /usr/local/cargo/bin/charms /usr/local/bin
CMD ["charms", "server"]
