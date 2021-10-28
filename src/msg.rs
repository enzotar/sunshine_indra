use crate::error::{Error, Result};
use indradb::{EdgeKey, Type};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

// msg
pub enum Msg {
    CreateGraph(JsonValue),
    ListGraphs,
    // DeleteGraph(GraphId),
    CreateVertex(CreateVertex),
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

pub struct MsgWithGraphId {
    pub msg: Msg,
    pub graph_id: Option<GraphId>,
}

#[derive(Debug)]
pub struct Graph {
    pub vertices: Vec<VertexInfo>,
    pub state_id: String,
}

pub type GraphId = String;

pub enum Query {
    ReadGraph((GraphId, StateId)),
}

pub type StateId = String;

pub type VertexId = String;

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

pub type EdgeInfo = JsonValue;

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

#[derive(Debug)]
pub enum Reply {
    Id(String),
    VertexInfoList(Vec<VertexInfo>),
    Error(String),
    VertexInfo(VertexInfo),
    EdgeInfo(EdgeInfo),
    Graph(Option<Graph>),
    Empty,
}

impl Reply {
    pub fn as_id(&self) -> Option<&str> {
        match self {
            Reply::Id(id) => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn from_graph(graph: Result<Option<Graph>>) -> Reply {
        match graph {
            Ok(graph) => Reply::Graph(graph),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

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

    pub fn from_vertex_info_list(info: Result<Vec<VertexInfo>>) -> Reply {
        match info {
            Ok(info) => Reply::VertexInfoList(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_edge_info(info: Result<EdgeInfo>) -> Reply {
        match info {
            Ok(info) => Reply::EdgeInfo(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }
}
