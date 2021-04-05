use serde::Deserialize;
use gotham_derive::{StateData, StaticResponseExtender};

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct PathExtractor {
    pub key: String,
}
