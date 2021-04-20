use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use gotham::middleware::state::StateMiddleware;

mod extractor;
use extractor::PathExtractor;

enum Bracket{
    Open,
    Closed
}

struct Interval{
    m: u64,
    bracket1: Bracket,
    val1: u64,
    bracket2: Bracket,
    val2: u64
}

impl Interval{
    fn new(m: u64, bracket1: Bracket, val1: u64, bracket2: Bracket, val2: u64) -> Self{
        Interval{m, bracket1, val1, bracket2, val2}
    }

    /// Needs improvement - right now this method uses a pretty inefficient way to check if a `val` lies between an `Interval. Selecting a `start` and and `end` and walking through each value between them isn't ideal, and we need  a faster way to do this.
    fn is_in_interval(&self, val: u64) -> bool{
        let mut start = match self.bracket1{
            Bracket::Open => self.val1 + 1,
            Bracket::Closed => self.val1
        };
        let end = match self.bracket2{
            Bracket::Open => self.val2 + 1,
            Bracket::Closed => self.val2
        };
        
        while start != end{
            if start == val{
                return true;
            }
            start += 1;
        }
        val == start
    }
}

struct FingerTable{
    start: usize,
    interval: Interval,
    node: u64
}

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
