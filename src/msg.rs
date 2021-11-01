use crate::{
    error::{Error, Result},
    store::Store,
};
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
    DeleteNode(Node),
    CreateEdge(Edge),
    UpdateEdge((Edge, JsonValue)),
    DeleteEdge(Edge),
    ReverseEdge(Edge),
}

// no graph id needed
pub enum Query {
    ListGraphs,
    ReadNode(NodeId),
    ReadEdge(Edge),
    ReadGraph(GraphId),
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub vertices: Vec<Node>,
    pub state_id: u64,
}

pub type GraphId = Uuid;

pub type NodeId = Uuid;
pub type EdgeId = Uuid;

// pub struct CreateNode {
//     pub node_type: String,
//     pub properties: JsonValue,
// }

#[derive(Debug, Clone, Default)]
pub struct Node {
    pub node_id: NodeId,
    pub properties: JsonValue,
    pub outbound_edges: Option<Vec<Edge>>,
    pub inbound_edges: Option<Vec<Edge>>,
}

impl Node {
    pub fn from_properties(properties: JsonValue) -> Self {
        Self {
            properties,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Edge {
    pub edge_type: String,
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub properties: JsonValue,
}

impl Edge {}

// pub type Edge = JsonValue;

// #[derive(Debug, Clone)]
// pub struct EdgeId {
//     pub from: NodeId,
//     pub to: NodeId,
//     pub edge_type: String,
// }

impl From<NodeId> for Node {
    fn from(_: NodeId) -> Self {
        todo!()
    }
    // fn from(node_id: NodeId) -> Self {
    //     let trans = Store::transaction()?;
    //     // let uuid = node_id;

    //     let query = SpecificVertexQuery::single(node_id);

    //     // let vertex_query: VertexQuery = query.clone().into();

    //     let outbound_query = query.clone().outbound();

    //     let inbound_query = query.clone().inbound();

    //     let mut properties = trans
    //         .get_all_vertex_properties(VertexQuery::Specific(query))
    //         .map_err(Error::GetVertices)?;
    //     assert_eq!(properties.len(), 1);

    //     let properties = properties.pop().unwrap().props.pop().unwrap().value;

    //     let outbound_edges = Some(
    //         trans
    //             .get_edges(outbound_query)
    //             .map_err(Error::GetEdgesOfVertex)?
    //             .into_iter()
    //             .map(|edge| Edge::from(edge.key))
    //             .collect(),
    //     );

    //     let inbound_edges = Some(
    //         trans
    //             .get_edges(inbound_query)
    //             .map_err(Error::GetEdgesOfVertex)?
    //             .into_iter()
    //             .map(|edge| Edge::from(edge.key))
    //             .collect(),
    //     );
    //     let node = Node {
    //         node_id,
    //         outbound_edges,
    //         inbound_edges,
    //         properties,
    //     };
    //     dbg!(node.clone());

    //     Ok(node)
    //     Self {
    //         node_id: todo!(),
    //         properties: todo!(),
    //         outbound_edges: todo!(),
    //         inbound_edges: todo!(),
    //     }
    // }
}

impl From<EdgeKey> for Edge {
    fn from(edge_key: EdgeKey) -> Self {
        Self {
            from: edge_key.outbound_id,
            to: edge_key.inbound_id,
            edge_type: edge_key.t.0,
            ..Default::default()
        }
    }
}

impl TryInto<EdgeKey> for Edge {
    type Error = Error;

    fn try_into(self) -> Result<EdgeKey> {
        Ok(EdgeKey {
            outbound_id: self.from,
            inbound_id: self.to,
            t: Type(self.edge_type),
        })
    }
}

#[derive(Debug, Clone)]
#[must_use = "this `Reply` may be an `Error` variant, which should be handled"]
pub enum Reply {
    Id(String),
    NodeList(Vec<Node>),
    Error(String),
    Node(Node),
    Edge(Edge),
    Graph(Graph),
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

    pub fn from_graph(result: Result<Graph>) -> Reply {
        match result {
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

    pub fn from_node(result: Result<Node>) -> Reply {
        match result {
            Ok(node) => Reply::Node(node),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_empty(val: Result<()>) -> Reply {
        match val {
            Ok(_) => Reply::Empty,
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_node_list(info: Result<Vec<Node>>) -> Reply {
        match info {
            Ok(info) => Reply::NodeList(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    pub fn from_edge(result: Result<Edge>) -> Reply {
        match result {
            Ok(edge) => Reply::Edge(edge),
            Err(e) => Reply::Error(e.to_string()),
        }
    }
}
