use std::collections::HashMap;
use sunshine_core::msg::Properties;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub data: Data,
    pub extensions: Extensions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub q: Vec<Node>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub uid: String,
    pub indra_id: String,
    #[serde(flatten)]
    pub properties: Properties,
    pub link: Option<Vec<Node>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extensions {
    pub server_latency: ServerLatency,
    pub txn: Txn,
    pub metrics: Metrics,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerLatency {
    pub parsing_ns: i64,
    pub processing_ns: i64,
    pub encoding_ns: i64,
    pub assign_timestamp_ns: i64,
    pub total_ns: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Txn {
    pub start_ts: i64,
    pub hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub num_uids: NumUids,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumUids {
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
    // pub field: Option<i64>,
    // #[serde(rename = "_total")]
    // pub total: i64,
    // pub action: Option<i64>,
    // pub display: i64,
    // pub inline_display: i64,
    // pub link: i64,
    // pub name: i64,
    // pub options: i64,
    // pub selection_mode: i64,
    // pub uid: i64,
    // pub validation: i64,
}
