FROM rust:latest
WORKDIR /
COPY . .
RUN rm -rf ./target
RUN cargo build --release
CMD ["cargo", "run", "--release"]