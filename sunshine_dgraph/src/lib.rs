use serde_json::Value as JsonValue;
use uuid::Uuid;

mod queries;
mod responses;

use queries::*;
use responses::node::{Node as DNode, Root};

use sunshine_core::msg::{
    CreateEdge, Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId,
    Query, RecreateNode, Reply,
};

use sunshine_core::{Error, Result};

#[tokio::main]
pub async fn query() -> std::result::Result<DNode, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // let query_by_uid = |uid: &str| {
    //     format!(
    //         r#"{{
    //             find(func: uid({}))  @recurse{{
    //                 uid
    //                 name
    //                 display
    //                 inlineDisplay
    //                 validation
    //                 action
    //                 link
    //                 options
    //                 selectionMode
    //             }}
    //         }}"#,
    //         uid
    //     )
    // };

    //mutate?commitNow=true

    let url = "https://quiet-leaf.us-west-2.aws.cloud.dgraph.io/query?=";
    let uid = "0x170f16be";

    let res = client
        .post(url)
        .body(query_by_uid(uid))
        .header(
            "x-auth-token",
            "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=",
        )
        .header("Content-Type", "application/graphql+-")
        .send()
        .await?;

    let t: Root = res.json().await?;

    let root_node: &DNode = &t.data.find.first().unwrap();

    dbg!(t.clone());
    // rid::log_debug!("Got query {:#?}", &root_node);

    Ok(root_node.clone())
}

#[async_trait::async_trait]
impl sunshine_core::Store for Store {
    fn undo_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.undo
    }

    fn redo_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.redo
    }

    fn history_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.history
    }

    async fn update_state_id(&self, graph_id: Uuid) -> Result<()> {
        todo!();
    }

    async fn create_graph(&self, properties: JsonValue) -> Result<(Msg, GraphId)> {
        todo!();
    }

    async fn list_graphs(&self) -> Result<Vec<Node>> {
        todo!();
    }

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        todo!();
    }

    async fn create_node(
        &self,
        (graph_id, properties): (GraphId, JsonValue),
    ) -> Result<(Msg, NodeId)> {
        todo!();
    }

    async fn create_graph_root(&self, properties: JsonValue) -> Result<NodeId> {
        todo!();
    }

    async fn read_node(&self, node_id: NodeId) -> Result<Node> {
        todo!();
    }

    async fn update_node(
        &self,
        (node_id, value): (NodeId, JsonValue),
        graph_id: GraphId,
    ) -> Result<Msg> {
        todo!();
    }

    async fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn recreate_edge(&self, edge: Edge, properties: JsonValue) -> Result<()> {
        todo!();
    }

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Msg, EdgeId)> {
        todo!();
    }

    async fn read_edge_properties(&self, msg: Edge) -> Result<JsonValue> {
        todo!();
    }

    async fn update_edge(
        &self,
        (edge, properties): (Edge, JsonValue),
        graph_id: GraphId,
    ) -> Result<Msg> {
        todo!();
    }

    async fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }
}

struct Store {
    undo: Vec<Msg>,
    redo: Vec<Msg>,
    history: Vec<Msg>,
}
