FROM rust:1.89-bookworm AS build

WORKDIR /src

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build /src/target/release/strand /usr/local/bin/strand

ENTRYPOINT ["strand"]
