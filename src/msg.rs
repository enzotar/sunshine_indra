use crate::error::{Error, Result};
use indradb::{EdgeKey, Type};
use serde_json::Value as JsonValue;
use std::convert::TryFrom;
use uuid::Uuid;

#[derive(Clone)]
pub enum Msg {
    MutateState(MutateState),
    Query(Query),
    CreateGraph(JsonValue),
    DeleteGraph(GraphId),
    Undo,
}

#[derive(Clone)]
pub struct MutateState {
    pub kind: MutateStateKind,
    pub graph_id: GraphId,
}
#[derive(Clone)]
pub enum MutateStateKind {
    CreateNode(JsonValue),
    RecreateNode(RecreateNode),
    UpdateNode((NodeId, JsonValue)),
    DeleteNode(NodeId),
    CreateEdge(CreateEdge),
    UpdateEdge((Edge, JsonValue)),
    DeleteEdge(Edge),
}

#[derive(Clone)]
pub enum Query {
    ListGraphs,
    ReadNode(NodeId),
    ReadEdgeProperties(Edge),
    ReadGraph(GraphId),
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub state_id: u64,
}

pub type GraphId = Uuid;
pub type NodeId = Uuid;

pub type EdgeId = Uuid;

#[derive(Debug, Clone, Default)]
pub struct RecreateNode {
    pub node_id: NodeId,
    pub properties: JsonValue,
    pub edges: Vec<(Edge, JsonValue)>,
}

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub node_id: NodeId,
    pub properties: JsonValue,
    pub outbound_edges: Vec<Edge>,
    pub inbound_edges: Vec<Edge>,
}

// struct NodeProperties {
//     #[serde(flatten)]
//     extra: JsonValue,
//     name: String,
// }

#[derive(Debug, Clone, Copy, Default)]
pub struct Edge {
    pub id: EdgeId, // EdgeType
    pub from: NodeId,
    pub to: NodeId,
}

#[derive(Debug, Clone, Default)]
pub struct CreateEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub properties: JsonValue,
}

impl TryFrom<EdgeKey> for Edge {
    type Error = Error;

    fn try_from(edge_key: EdgeKey) -> Result<Self> {
        Ok(Self {
            from: edge_key.outbound_id,
            to: edge_key.inbound_id,
            id: Uuid::parse_str(edge_key.t.0.as_str())?,
        })
    }
}

impl From<Edge> for EdgeKey {
    fn from(edge: Edge) -> EdgeKey {
        EdgeKey {
            outbound_id: edge.from,
            inbound_id: edge.to,
            t: Type(edge.id.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Reply {
    Id(Uuid),
    NodeList(Vec<Node>),
    Node(Node),
    Edge(Edge),
    Graph(Graph),
    Json(JsonValue),
    Empty,
}

impl Reply {
    pub fn into_edge(self) -> Option<Edge> {
        match self {
            Reply::Edge(edge) => Some(edge),
            _ => None,
        }
    }

    pub fn into_node(self) -> Option<Node> {
        match self {
            Reply::Node(node) => Some(node),
            _ => None,
        }
    }

    pub fn into_graph(self) -> Option<Graph> {
        match self {
            Reply::Graph(graph) => Some(graph),
            _ => None,
        }
    }

    pub fn as_id(&self) -> Option<Uuid> {
        match self {
            Reply::Id(id) => Some(*id),
            _ => None,
        }
    }
}
