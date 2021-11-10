use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sunshine_core::msg::Properties;

#[derive(Serialize, Deserialize, Debug)]
pub struct Mutate {
    pub set: MutateCreateGraph,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MutateCreateGraph {
    pub indra_id: String,
    pub state_id: i32,
    pub is_graph_root: bool,
    #[serde(flatten)]
    pub properties: Properties,
}

// pub struct Upsert {
//     pub query: Query
//     pub set:,
// }

// pub struct Query {
//     pub query: String,
// }

// pub struct MutateNode{
//     pub uid
// }
