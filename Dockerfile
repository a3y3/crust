FROM rust:1.50
WORKDIR /crust
# The following command creates a dummy Rust file, which forces Docker to compile dependencies in the build
# This allows Docker to cache the dependencies, so compilation happens only once instead of at every `docker run`
COPY src/dummy.rs ./src/dummy.rs
COPY Cargo.toml .
RUN sed -i 's#src/main.rs#src/dummy.rs#' Cargo.toml
RUN cargo build
RUN sed -i 's#src/dummy.rs#src/main.rs#' Cargo.toml
#Resume normal build. Since the above lines weren't changed, Docker will use the cached dependencies!
COPY ./src ./src
COPY ./assets ./assets
RUN cargo build
EXPOSE 8000
ENTRYPOINT ["cargo" ,"run", "--bin", "crust"]