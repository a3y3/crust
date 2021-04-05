// Ignore me, I'm an empty Rust file that forces Docker to build the project dependencies during `docker build`
// This allows Docker to cache the dependencies, so compilation happens only once instead of at every `docker run`
fn main() {}
