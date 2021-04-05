use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham_derive::{StateData, StaticResponseExtender};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    name: String,
}

pub fn echo(state: State) -> (State, String) {
    let message = {
        let product = PathExtractor::borrow_from(&state);
        format!("Product: {}", product.name)
    };

    (state, message)
}

pub fn get_successor(state: State) -> (State, &'static str) {
    (state, "node0")
}

fn router() -> Router {
    build_simple_router(|route| {
        route.get("/successor").to(get_successor);
        route
            .get("/echo/:name")
            .with_path_extractor::<PathExtractor>()
            .to(echo);
    })
}

pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

mod tests;
