FROM rust:1.50
WORKDIR /crust
# The following command creates a dummy Rust file, which forces Docker to compile dependencies in the build
# This allows Docker to cache the dependencies, so compilation happens only once instead of at every `docker run`
COPY docker/dummy.rs ./docker/dummy.rs
COPY Cargo.toml .
RUN cargo build --bin docker-only
COPY . .
RUN cargo build
ENTRYPOINT [ "cargo" ,"run", "--bin", "crust"]