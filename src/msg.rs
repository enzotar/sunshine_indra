use crate::error::{Error, Result};
use indradb::{EdgeKey, Type};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

pub enum Msg {
    MutateState(MutateState),
    Query(Query),
    CreateGraph(JsonValue),
}

pub struct MutateState {
    pub kind: MutateStateKind,
    pub graph_id: GraphId,
}
pub enum MutateStateKind {
    DeleteGraph(GraphId),
    CreateNode(Node),
    UpdateNode((NodeId, JsonValue)),
    DeleteNode(NodeId),
    CreateEdge(CreateEdge),
    UpdateEdge((EdgeId, JsonValue)),
    DeleteEdge(EdgeId),
    ReverseEdge(EdgeId),
}

// no graph id needed
pub enum Query {
    ListGraphs,
    ReadNode(NodeId),
    ReadEdge(EdgeId),
    ReadGraph(GraphId),
}

#[derive(Debug)]
pub struct Graph {
    pub vertices: Vec<Node>,
    pub state_id: u64,
}

pub type GraphId = String;

pub type NodeId = String;

// pub struct CreateNode {
//     pub node_type: String,
//     pub properties: JsonValue,
// }

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub node_id: Uuid,
    pub properties: JsonValue,
    pub outbound_edges: Option<Vec<EdgeId>>,
    pub inbound_edges: Option<Vec<EdgeId>>,
}

impl Node {
    pub fn from_properties(properties: JsonValue) -> Self {
        Self {
            properties,
            ..Default::default()
        }
    }
}

pub struct CreateEdge {
    pub directed: bool,
    pub from: NodeId,
    pub edge_type: String,
    pub to: NodeId,
    pub properties: JsonValue,
}

pub type EdgeInfo = JsonValue;

#[derive(Debug, Clone)]
pub struct EdgeId {
    pub from: NodeId,
    pub to: NodeId,
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
#[must_use = "this `Reply` may be an `Error` variant, which should be handled"]
pub enum Reply {
    Id(String),
    NodeList(Vec<Node>),
    Error(String),
    Node(Node),
    EdgeInfo(EdgeInfo),
    Graph(Graph),
    Empty,
}

impl Reply {
    pub fn into_edge_info(self) -> Option<EdgeInfo> {
        match self {
            Reply::EdgeInfo(edge_info) => Some(edge_info),
            _ => None,
        }
    }

    pub fn into_node_info(self) -> Option<Node> {
        match self {
            Reply::Node(node_info) => Some(node_info),
            _ => None,
        }
    }

    pub fn into_graph(self) -> Option<Graph> {
        match self {
            Reply::Graph(graph) => Some(graph),
            _ => None,
        }
    }

    pub fn as_error(&self) -> std::result::Result<(), &str> {
        match self {
            Reply::Error(e) => Err(e.as_str()),
            _ => Ok(()),
        }
    }

    pub fn as_id(&self) -> Option<&str> {
        match self {
            Reply::Id(id) => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn from_graph(graph: Result<Graph>) -> Reply {
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

    pub fn from_node(node: Result<&Node>) -> Reply {
        match node {
            Ok(node) => Reply::Node(*node),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_empty(val: Result<()>) -> Reply {
        match val {
            Ok(_) => Reply::Empty,
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_vertex_info(info: Result<Node>) -> Reply {
        match info {
            Ok(info) => Reply::Node(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_vertex_info_list(info: Result<Vec<Node>>) -> Reply {
        match info {
            Ok(info) => Reply::NodeList(info),
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
