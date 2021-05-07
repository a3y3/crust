use gotham::handler::HandlerError;
use gotham::helpers::http::response::create_response;
use gotham::hyper::{body, Body, Response, StatusCode};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use mime::TEXT_PLAIN;
use simple_error::SimpleError;
use std::net::IpAddr;
use url::form_urlencoded;

mod extractor;
use extractor::PathExtractor;
mod lib;
use lib::initialize_node;
use lib::start_stabilize_thread;
use lib::ChordNode;

const PORT: usize = 8000;
/// returns the immediate successor of this node (GET /successor/)
fn get_successor(state: State) -> (State, String) {
    let node = ChordNode::borrow_from(&state);
    let successor = node.get_successor();
    (state, successor.to_string())
}

async fn update_successor(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let data = form_urlencoded::parse(&full_body).into_owned();
    let mut ip = String::new();
    for (key, value) in data {
        if key == "ip" {
            ip = value;
        } else {
            let error = SimpleError::new(format!("Invalid key {}, expected key: ip.", key));
            let handler_error = HandlerError::from(error).with_status(StatusCode::BAD_REQUEST);
            return Err(handler_error);
        }
    }

    let node = state.borrow_mut::<ChordNode>();
    println!("Will update my successor to {}", ip);
    node.update_successor(ip.parse()?);
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    Ok(response)
}

/// returns the immediate successor of this node (GET /predecessor/)
fn get_predecessor(state: State) -> (State, String) {
    let node = ChordNode::borrow_from(&state);
    let predecessor = node.get_predecessor();
    (state, predecessor.to_string())
}

async fn update_predecessor(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let data = form_urlencoded::parse(&full_body).into_owned();
    let mut ip = String::new();
    for (key, value) in data {
        if key == "ip" {
            ip = value;
        }
    }
    if ip == "" {
        let error = SimpleError::new(format!("Invalid data, expected ip:address"));
        let handler_error = HandlerError::from(error).with_status(StatusCode::BAD_REQUEST);
        return Err(handler_error);
    }

    let node = state.borrow::<ChordNode>();
    println!("Will update my predecessor to {}", ip);
    node.update_predecessor(ip.parse()?);
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    Ok(response)
}

/// calculates the successor(key) and returns the IP address and ID of the node.
async fn calculate_successor(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let id = &PathExtractor::borrow_from(&state).key;
    println!("calculate_successor: calculating successor for: {}", id);
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

async fn info(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let resp = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, node.info());
    Ok(resp)
}

async fn get_ring(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let ring = node.ring_info().await?;
    let resp = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, ring);
    Ok(resp)
}

async fn update_finger_table(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let data = form_urlencoded::parse(&full_body).into_owned();
    let mut n = String::new();
    let mut i = String::new();
    for (key, value) in data {
        if key == "n" {
            n = value;
        } else if key == "i" {
            i = value;
        } else {
            let error = SimpleError::new(format!("Invalid key {}, expected key: n or i.", key));
            let handler_error = HandlerError::from(error).with_status(StatusCode::BAD_REQUEST);
            return Err(handler_error);
        }
    }
    let s: IpAddr = n.parse()?;
    let i: u64 = i.parse()?;
    let node = state.borrow_mut::<ChordNode>();
    node.update_finger_table(s, i).await?;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    Ok(response)
}

async fn notify(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let data = form_urlencoded::parse(&full_body).into_owned();
    let mut n = String::new();
    for (key, value) in data {
        if key == "n" {
            n = value;
        }
    }
    if n == "" {
        let error = SimpleError::new(format!(
            "Invalid data, expected n: IP address of node that created this PATCH request"
        ));
        let handler_error = HandlerError::from(error).with_status(StatusCode::BAD_REQUEST);
        return Err(handler_error);
    }

    let node = state.borrow::<ChordNode>();
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    node.notify(n.parse()?).await;
    Ok(response)
}
/// add a new key-value pair to the DHT (supplied as POST to /key/)
fn create_value(_state: State) -> (State, String) {
    unimplemented!()
}

/// returns the value corresponsing to the key in (GET /key/:key)
fn get_value(_state: State) -> (State, String) {
    unimplemented!()
}

/// update the value corresponding to the supplied key (PATCH /key/:key)
fn update_value(_state: State) -> (State, String) {
    unimplemented!()
}

/// delete a key value pair (DELETE /key/:key)
fn delete_value(_state: State) -> (State, String) {
    unimplemented!()
}

fn router(chord: ChordNode) -> Router {
    let middleware = StateMiddleware::new(chord);
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.get("/ring").to_async_borrowing(get_ring);
        route.get("/").to_file("assets/index.html");
        route.scope("/successor", |route| {
            route.get("/").to(get_successor);
            route.patch("/").to_async_borrowing(update_successor);
            route
                .get("/:key")
                .with_path_extractor::<PathExtractor>()
                .to_async_borrowing(calculate_successor);
            route
                .get("/cpf/:key")
                .with_path_extractor::<PathExtractor>()
                .to(closest_preceding_finger);
        });
        route.scope("/predecessor", |route| {
            route.get("/").to(get_predecessor);
            route.patch("/").to_async_borrowing(update_predecessor);
        });
        route.get("/info").to_async_borrowing(info);
        route
            .patch("/fingertable")
            .to_async_borrowing(update_finger_table);
        route.patch("/notify").to_async_borrowing(notify);
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

fn main() {
    let chord = initialize_node();
    start_stabilize_thread(chord.clone());
    let addr = format!("0.0.0.0:{}", PORT);
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(chord));
}

mod tests;
