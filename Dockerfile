FROM rust:alpine AS base
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app

FROM base AS builder
COPY . .
RUN cargo install --locked --path . --bin charms

FROM alpine AS runtime
COPY --from=builder /usr/local/cargo/bin/charms /usr/local/bin
CMD ["charms", "server"]
