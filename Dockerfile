FROM rust:alpine3.14 as builder

WORKDIR /app

RUN apk add musl-dev

#COPY ./.cargo .cargo
#COPY ./vendor vendor
COPY Cargo.toml Cargo.lock ./
COPY ./src src

# build with x86_64-unknown-linux-musl to make it runs on alpine.
RUN cargo install --path /app --target=x86_64-unknown-linux-musl


FROM alpine:3.14
COPY --from=builder /usr/local/cargo/bin/* /usr/local/bin/

# Let's run as 1000 to keep FS access simple
RUN addgroup -g 1000 user
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "$(pwd)" \
    --ingroup "user" \
    --no-create-home \
    --uid 1000 \
    user

USER user
WORKDIR /
ENTRYPOINT ["/usr/local/bin/mqtt-saver"]
VOLUME /data
