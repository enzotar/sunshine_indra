use std::collections::HashMap;
use sunshine_core::msg::Properties;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Query
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRoot {
    pub data: HashMap<String, Vec<Node>>,
    pub extensions: Extensions,
}

/* Upsert Reponse Example
{
  "data": {
    "code": "Success",
    "message": "Done",
    "queries": {
      "q": [
        {
          "uid": "0xfffd8d6aac77a6a1",
          "state_id": 0,
          "val(n)": 1
        },
        {
          "uid": "0xfffd8d6aac79e2ba",
          "state_id": 0,
          "val(n)": 1
        },
        {
          "uid": "0xfffd8d6aac79e2d1",
          "state_id": 0,
          "val(n)": 1
        },
        {
          "uid": "0xfffd8d6aac79ef90",
          "state_id": 0,
          "val(n)": 1
        },
        {
          "uid": "0xfffd8d6aac79f069",
          "state_id": 0,
          "val(n)": 1
        }
      ]
    },
    "uids": {}
  },
  "extensions": {
    "server_latency": {
      "parsing_ns": 49891,
      "processing_ns": 186608916,
      "encoding_ns": 39290,
      "assign_timestamp_ns": 35354811,
      "total_ns": 222280890
    },
    "txn": {
      "start_ts": 13482720,
      "commit_ts": 13482721,
      "preds": [
        "1-53-state_id"
      ],
      "hash": "57068807604791c5b923a55334bb08c623f98733e5887973af2478b5708863e8"
    }
  }
}
*/
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpsertRoot {
    pub data: UpsertData,
    pub extensions: Extensions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpsertData {
    pub code: String,
    pub message: String,
    pub queries: HashMap<String, Vec<Node>>,
    pub uids: Option<HashMap<String, String>>,
}

/*
{
  "data": {
    "code": "Success",
    "message": "Done",
    "queries": null,
    "uids": {
      "dg.452357893.36": "0xfffd8d6aac7a3e32"
    }
  },
  "extensions": {
    "server_latency": {
      "parsing_ns": 42985,
      "processing_ns": 170493890,
      "assign_timestamp_ns": 1862749,
      "total_ns": 172705345
    },
    "txn": {
      "start_ts": 13519708,
      "commit_ts": 13519710,
      "preds": [
        "1-53-indra_id",
        "1-53-state_id"
      ],
      "hash": "c694763165f1cf75ef1d46acce3237f6c8a59ad6cbb0e2d93186d820f1256f63"
    }
  }
}
*/
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateRoot {
    pub data: MutateData,
    pub extensions: Extensions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateData {
    pub code: String,
    pub message: String,
    pub uids: HashMap<String, String>,
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
    pub metrics: Option<Metrics>,
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
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
    // pub num_uids: NumUids,
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
