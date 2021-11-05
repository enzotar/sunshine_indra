use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub data: Data,
    pub extensions: Extensions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub find: Vec<Node>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub uid: String,

    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
    pub link: Option<Vec<Node>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    #[serde(rename = "server_latency")]
    pub server_latency: ServerLatency,
    pub txn: Txn,
    pub metrics: Metrics,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerLatency {
    #[serde(rename = "parsing_ns")]
    pub parsing_ns: i64,
    #[serde(rename = "processing_ns")]
    pub processing_ns: i64,
    #[serde(rename = "encoding_ns")]
    pub encoding_ns: i64,
    #[serde(rename = "assign_timestamp_ns")]
    pub assign_timestamp_ns: i64,
    #[serde(rename = "total_ns")]
    pub total_ns: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Txn {
    #[serde(rename = "start_ts")]
    pub start_ts: i64,
    pub hash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    #[serde(rename = "num_uids")]
    pub num_uids: NumUids,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumUids {
    #[serde(rename = "")]
    pub field: i64,
    #[serde(rename = "_total")]
    pub total: i64,
    pub action: i64,
    pub display: i64,
    pub inline_display: i64,
    pub link: i64,
    pub name: i64,
    pub options: i64,
    pub selection_mode: i64,
    pub uid: i64,
    pub validation: i64,
}
