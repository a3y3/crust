use gotham::handler::HandlerError;
use gotham::helpers::http::response::create_response;
use gotham::hyper::{Body, Response, StatusCode};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};

use mime::TEXT_PLAIN;

mod extractor;
use extractor::PathExtractor;
mod lib;
use lib::initialize_node;
use lib::ChordNode;

const PORT: usize = 8000;

/// returns the immediate successor of this node (GET /successor/)
fn get_successor(state: State) -> (State, String) {
    let node = ChordNode::borrow_from(&state);
    let successor = node.get_successor();
    (state, successor.to_string())
}

/// calculates the successor(key) and returns the IP address and ID of the node.
async fn calculate_successor(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let id = &PathExtractor::borrow_from(&state).key;
    let res = node.calculate_successor(id).await?;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, res.to_string());
    Ok(response)
}

fn closest_preceding_finger(state: State) -> (State, String) {
    let node = ChordNode::borrow_from(&state);
    let id = &PathExtractor::borrow_from(&state).key;
    let res = node.closest_preceding_finger(id);
    (state, res.to_string())
}

/// add a new key-value pair to the DHT (supplied as POST to /key/)
fn create_value(_state: State) -> (State, String) {
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
fn update_value(_state: State) -> (State, String) {
    unimplemented!()
}

/// delete a key value pair (DELETE /key/:key)
fn delete_value(_state: State) -> (State, String) {
    unimplemented!()
}

async fn next_node(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let ip = &PathExtractor::borrow_from(&state).key;
    println!("ip: {}", ip);
    let resp = reqwest::get(format!("http://{}:{}/successor/", ip, PORT)).await?;
    let result = resp.text().await?;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, result);
    Ok(response)
}

fn router(chord: ChordNode) -> Router {
    let middleware = StateMiddleware::new(chord);
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.scope("/successor", |route| {
            route.get("/").to(get_successor);
            route
                .get("/:key")
                .with_path_extractor::<PathExtractor>()
                .to_async_borrowing(calculate_successor);
            route.get("/cfp/:key").to(closest_preceding_finger);
        });
        route
            .get("/comms/check/:key")
            .with_path_extractor::<PathExtractor>()
            .to_async_borrowing(next_node);

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
