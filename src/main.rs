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

/// Update a node's successor to a new node (PATCH /successor/)
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

/// update a node's predecessor pointer (PATCH /predecessor/)
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

/// Find the closest predecessing finger for a given id (GET /successor/cfp/:id)
fn closest_preceding_finger(state: State) -> (State, String) {
    let node = ChordNode::borrow_from(&state);
    let id = &PathExtractor::borrow_from(&state).key;
    let res = node.closest_preceding_finger(id);
    (state, res.to_string())
}

/// return all information about this node (GET /info/)
async fn info(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let resp = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, node.info());
    Ok(resp)
}

/// return a JSON representing the structure of the Chord ring (GET /ring/)
async fn get_ring(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let ring = node.ring_info().await?;
    let resp = create_response(&state, StatusCode::OK, mime::APPLICATION_JSON, ring);
    Ok(resp)
}

/// update the finger tables of a node (PATCH /fingertable/) 
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

/// Notify a node that there might be a better predecessor (PATCH /notify/)
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
    node.notify(n.parse()?).await;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    Ok(response)
}

/// add a new key to the DST (supplied as POST to /key/)
async fn insert(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let key = get_key_from_body(state).await?;
    let node = state.borrow::<ChordNode>();
    let inserted_at_id = node.insert(key).await?;
    let response = create_response(
        &state,
        StatusCode::OK,
        TEXT_PLAIN,
        format!("{}", inserted_at_id),
    );
    Ok(response)
}

/// Adds a key to a node's replica_state field. (POST /replica/)
async fn insert_replica(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let key = get_key_from_body(state).await?;
    let node = state.borrow::<ChordNode>();
    node.insert_replica(key);
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, "".to_string());
    Ok(response)
}

/// Return the field `key` from a POST body.
async fn get_key_from_body(state: &mut State) -> Result<String, HandlerError> {
    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let data = form_urlencoded::parse(&full_body).into_owned();
    let mut key = String::new();
    for (k, v) in data {
        if k == "key" {
            key = v;
        } else {
            let error = SimpleError::new(format!("Invalid key {}, expected key: 'key'", k));
            let handler_error = HandlerError::from(error).with_status(StatusCode::BAD_REQUEST);
            return Err(handler_error);
        }
    }
    Ok(key)
}

/// returns the value corresponsing to the key in (GET /key/:key)
async fn contains(state: &mut State) -> Result<Response<Body>, HandlerError> {
    let node = ChordNode::borrow_from(&state);
    let key = &PathExtractor::borrow_from(&state).key;
    let contains = node.contains(key).await?;
    let response = create_response(&state, StatusCode::OK, TEXT_PLAIN, contains.to_string());
    Ok(response)
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
            route.post("/").to_async_borrowing(insert);
            route
                .get("/:key")
                .with_path_extractor::<PathExtractor>()
                .to_async_borrowing(contains);
        });
        route.post("/replica").to_async_borrowing(insert_replica);
    })
}

fn main() {
    let chord = initialize_node();
    start_stabilize_thread(chord.clone());
    let addr = format!("0.0.0.0:{}", PORT);
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(chord));
}
