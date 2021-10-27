use crate::error::{Error, Result};
use indradb::{EdgeKey, Type};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

// msg
pub enum Msg {
    CreateVertex((GraphId, CreateVertex)),
    ReadVertex(VertexId),
    UpdateVertex((VertexId, JsonValue)),
    DeleteVertex(VertexId),
    CreateEdge(CreateEdge),
    ReadEdge(EdgeId),
    UpdateEdge((EdgeId, JsonValue)),
    DeleteEdge(EdgeId),
    ReverseEdge(EdgeId),
    Query(Query),
}

pub type GraphId = String;

pub enum Query {
    Graph(String),
}

pub struct CreateVertex {
    pub vertex_type: String,
    pub properties: JsonValue,
}

#[derive(Debug)]
pub struct VertexInfo {
    pub outbound_edges: Vec<EdgeId>,
    pub inbound_edges: Vec<EdgeId>,
    pub properties: JsonValue,
}

pub struct CreateEdge {
    pub directed: bool,
    pub from: VertexId,
    pub edge_type: String,
    pub to: VertexId,
    pub properties: JsonValue,
}

pub type VertexId = String;

#[derive(Debug, Clone)]
pub struct EdgeId {
    pub from: VertexId,
    pub to: VertexId,
    pub edge_type: String,
}

impl From<EdgeKey> for EdgeId {
    fn from(edge_key: EdgeKey) -> EdgeId {
        EdgeId {
            from: edge_key.outbound_id.to_string(),
            to: edge_key.inbound_id.to_string(),
            edge_type: edge_key.t.0,
        }
    }
}

impl TryInto<EdgeKey> for EdgeId {
    type Error = Error;

    fn try_into(self) -> Result<EdgeKey> {
        Ok(EdgeKey {
            outbound_id: Uuid::parse_str(&self.from)?,
            inbound_id: Uuid::parse_str(&self.to)?,
            t: Type(self.edge_type),
        })
    }
}

pub type EdgeInfo = JsonValue;

#[derive(Debug)]
pub enum Reply {
    // DbState(DbState),
    Id(String),
    Error(String),
    VertexInfo(VertexInfo),
    EdgeInfo(EdgeInfo),
    Empty,
    Graph(Graph),
}

#[derive(Debug)]
pub struct Graph {
    pub vertices: Vec<VertexInfo>,
}

impl Reply {
    pub fn from_id(id: Result<String>) -> Reply {
        match id {
            Ok(id) => Reply::Id(id),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_empty(val: Result<()>) -> Reply {
        match val {
            Ok(_) => Reply::Empty,
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_vertex_info(info: Result<VertexInfo>) -> Reply {
        match info {
            Ok(info) => Reply::VertexInfo(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_edge_info(info: Result<EdgeInfo>) -> Reply {
        match info {
            Ok(info) => Reply::EdgeInfo(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_graph(graph: Result<Graph>) -> Reply {
        match graph {
            Ok(graph) => Reply::Graph(graph),
            Err(e) => Reply::Error(e.to_string()),
        }
    }
}
