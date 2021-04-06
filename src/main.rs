use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

mod extractor;
use extractor::PathExtractor;

/// returns the immediate successor of this node
pub fn get_successor(state: State) -> (State, &'static str) {
    (state, "node0")
}

/// add a new key-value pair to the DHT (supplied as POST to /key/)
pub fn create_value(state: State) -> (State, String) {
    unimplemented!()
}

/// returns the value corresponsing to the key in (GET /key/:key)
pub fn get_value(state: State) -> (State, String) {
    let key = {
        let data = PathExtractor::borrow_from(&state);
        format!("You entered: {}", data.key)
    };
    (state, key)
}

/// update the value corresponding to the supplied key (PATCH /key/:key)
pub fn update_value(state: State) -> (State, String) {
    unimplemented!()
}

/// delete a key value pair (DELETE /key/:key)
pub fn delete_value(state: State) -> (State, String) {
    unimplemented!()
}

fn router() -> Router {
    build_simple_router(|route| {
        route.get("/successor").to(get_successor);

        route.scope("/key", |route| {
            route.post("/").to(create_value);
            route
                .get("/:key") // GET /key/1234
                .with_path_extractor::<PathExtractor>()
                .to(get_value);
            route
                .patch("/:key") // PATCH /key/1234
                .with_path_extractor::<PathExtractor>()
                .to(update_value);
            route
                .delete("/:key") // DELETE /key/1234
                .with_path_extractor::<PathExtractor>()
                .to(delete_value);
        });
    })
}

pub fn main() {
    let addr = "0.0.0.0:8000";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

mod tests;
