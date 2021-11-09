use indradb::{EdgeKey, Type};
use serde_json::Value as JsonValue;
use std::convert::TryFrom;
use uuid::Uuid;

// this map can't contain Objects
pub type Properties = serde_json::Map<String, JsonValue>;

#[derive(Clone, Debug)]
pub enum Msg {
    MutateState(MutateState),
    Query(Query),
    CreateGraph(Properties),
    CreateGraphWithId(GraphId, Properties),
    DeleteGraph(GraphId),
    Undo,
    Redo,
}

#[derive(Clone, Debug)]
pub struct MutateState {
    pub kind: MutateStateKind,
    pub graph_id: GraphId,
}
#[derive(Clone, Debug)]
pub enum MutateStateKind {
    CreateNode(Properties),
    RecreateNode(RecreateNode),
    UpdateNode((NodeId, Properties)),
    DeleteNode(NodeId),
    CreateEdge(CreateEdge),
    UpdateEdge((Edge, Properties)),
    DeleteEdge(Edge),
}

#[derive(Clone, Debug)]
pub enum Query {
    ListGraphs,       // graph node list
    ReadNode(NodeId), //node properties and edges
    ReadEdgeProperties(Edge),
    ReadGraph(GraphId), //list of nodes[edges]
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
    pub properties: Properties,
    pub edges: Vec<(Edge, Properties)>,
}

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub node_id: NodeId,
    pub properties: Properties,
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
    pub properties: Properties,
}

impl TryFrom<EdgeKey> for Edge {
    type Error = uuid::Error;

    fn try_from(edge_key: EdgeKey) -> Result<Self, uuid::Error> {
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
    Properties(Properties),
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
