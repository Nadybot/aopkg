FROM docker.io/library/alpine:edge AS builder

RUN apk upgrade && \
    apk add curl gcc musl-dev dumb-init && \
    curl -sSf https://sh.rustup.rs | sh -s -- --profile minimal --default-toolchain nightly -y

WORKDIR /build

COPY Cargo.toml Cargo.lock ./

RUN mkdir src/
RUN echo 'fn main() {}' > ./src/main.rs
RUN source $HOME/.cargo/env && \
    cargo build --release

RUN rm -f target/release/deps/aopkg*
COPY ./src ./src
COPY ./templates ./templates

RUN source $HOME/.cargo/env && \
    cargo build --release && \
    cp target/release/aopkg /aopkg && \
    strip /aopkg

FROM scratch

COPY --from=builder /usr/bin/dumb-init /dumb-init
COPY --from=builder /aopkg /aopkg
COPY static static
COPY migrations migrations

ENTRYPOINT ["./dumb-init", "--"]
CMD ["./aopkg"]
