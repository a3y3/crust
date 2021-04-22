use gotham::handler::HandlerError;
use gotham::helpers::http::response::create_response;
use gotham::hyper::{Body, Response, StatusCode};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single_middleware;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

use mime::TEXT_PLAIN;

mod extractor;
use extractor::PathExtractor;
mod lib;
use lib::initialize_node;
use lib::Chord;

const PORT: usize = 8000;

/// returns the immediate successor of this node
fn get_successor(state: State) -> (State, String) {
    let chord = Chord::borrow_from(&state);
    let successor = chord.get_successor();
    (state, successor.to_string())
}

/// add a new key-value pair to the DHT (supplied as POST to /key/)
fn create_value(state: State) -> (State, String) {
    unimplemented!()
}

/// returns the value corresponsing to the key in (GET /key/:key)
fn get_value(state: State) -> (State, String) {
    let key = {
        let data = PathExtractor::borrow_from(&state);
        format!("You entered: {}", data.key)
    };
    (state, key)
}

/// update the value corresponding to the supplied key (PATCH /key/:key)
fn update_value(state: State) -> (State, String) {
    unimplemented!()
}

/// delete a key value pair (DELETE /key/:key)
fn delete_value(state: State) -> (State, String) {
    unimplemented!()
}

async fn next_node(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let ip = "20.40.60.80";
    let resp = reqwest::get(format!("{}:{}/successor/", ip, PORT)).await?;
    let result = resp.text().await?;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, result);
    Ok(response)
}

fn router(chord: Chord) -> Router {
    let middleware = StateMiddleware::new(chord);
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.get("/successor").to(get_successor);
        route.get("/nextnode").to_async_borrowing(next_node);

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
    let chord = initialize_node();
    let addr = format!("0.0.0.0:{}", PORT);
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(chord))
}

mod tests;
