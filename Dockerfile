FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev
RUN cargo build --release

FROM alpine

COPY --from=builder /build/target/release/sneakyservermetrics /usr/bin/sneakyservermetrics

CMD [ "/usr/bin/sneakyservermetrics" ]
