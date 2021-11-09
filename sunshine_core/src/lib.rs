use serde_json::Value as JsonValue;
use uuid::Uuid;

pub mod error;
pub mod msg;
pub mod properties;

pub use error::{Error, Result};

use msg::{
    CreateEdge, Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId,
    Properties, Query, RecreateNode, Reply,
};

#[derive(Debug)]
pub enum Operation {
    Undo,
    Redo,
    Other,
}

#[async_trait::async_trait]
pub trait Store: Send + Sync {
    fn undo_buf(&mut self) -> &mut Vec<Msg>;

    fn redo_buf(&mut self) -> &mut Vec<Msg>;

    fn history_buf(&mut self) -> &mut Vec<Msg>;

    async fn execute(&mut self, msg: msg::Msg) -> Result<msg::Reply> {
        self.execute_impl(msg, Operation::Other).await
    }

    async fn execute_impl(&mut self, msg: Msg, operation: Operation) -> Result<Reply> {
        let (reverse_msg, reply) = match msg.clone() {
            Msg::CreateGraph(properties) => self
                .create_graph(properties)
                .await
                .map(|(reverse_msg, node)| (Some(reverse_msg), Reply::Id(node)))?,
            Msg::CreateGraphWithId(uuid, properties) => self
                .create_graph_with_id(uuid, properties)
                .await
                .map(|(reverse_msg, node)| (Some(reverse_msg), Reply::Id(node)))?,
            Msg::MutateState(mutate_state) => self
                .execute_mutate_state(mutate_state)
                .await
                .map(|(reverse_msg, reply)| (Some(reverse_msg), reply))?,
            Msg::Query(read_only) => (None, self.execute_read_only(read_only).await?),
            Msg::DeleteGraph(_) => todo!(),
            Msg::Undo => {
                let reverse_msg = self.undo_buf().pop().ok_or(Error::UndoBufferEmpty)?;
                self.execute_impl(reverse_msg, Operation::Undo)
                    .await
                    .map(|reply| (None, reply))?
            }
            Msg::Redo => {
                let reverse_msg = self.redo_buf().pop().ok_or(Error::RedoBufferEmpty)?;
                self.execute_impl(reverse_msg, Operation::Redo)
                    .await
                    .map(|reply| (None, reply))?
            }
        };

        if let Some(reverse_msg) = reverse_msg {
            match operation {
                Operation::Other => {
                    self.redo_buf().clear();
                    self.undo_buf().push(reverse_msg);
                }
                Operation::Redo => self.undo_buf().push(reverse_msg),
                Operation::Undo => self.redo_buf().push(reverse_msg),
            }
        }

        self.history_buf().push(msg);

        Ok(reply)
    }

    async fn execute_mutate_state(&self, msg: MutateState) -> Result<(Msg, Reply)> {
        let MutateState { kind, graph_id } = msg;

        let (undo_msg, reply) = match kind {
            MutateStateKind::CreateNode(properties) => self
                .create_node((graph_id, properties))
                .await
                .map(|(undo_msg, node_id)| (undo_msg, Reply::Id(node_id)))?,
            MutateStateKind::RecreateNode(recreate_node) => {
                let node_id = recreate_node.node_id;
                self.recreate_node(recreate_node, graph_id)
                    .await
                    .map(|undo_msg| (undo_msg, Reply::Id(node_id)))?
            }
            MutateStateKind::UpdateNode((node_id, properties)) => self
                .update_node((node_id, properties), graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::DeleteNode(node_id) => self
                .delete_node(node_id, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::CreateEdge(edge) => self
                .create_edge(edge, graph_id)
                .await
                .map(|(undo_msg, edge_id)| (undo_msg, Reply::Id(edge_id)))?,
            MutateStateKind::UpdateEdge(edge) => self
                .update_edge(edge, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::DeleteEdge(edge) => self
                .delete_edge(edge, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
        };

        self.update_state_id(graph_id).await?;

        Ok((undo_msg, reply))
    }

    async fn execute_read_only(&self, msg: Query) -> Result<Reply> {
        match msg {
            Query::ReadEdgeProperties(msg) => {
                self.read_edge_properties(msg).await.map(Reply::Properties)
            }
            Query::ReadNode(msg) => self.read_node(msg).await.map(Reply::Node),
            Query::ReadGraph(read_graph) => self.read_graph(read_graph).await.map(Reply::Graph),
            Query::ListGraphs => self.list_graphs().await.map(Reply::NodeList),
        }
    }

    async fn update_state_id(&self, graph_id: GraphId) -> Result<()>;

    async fn create_graph(&self, properties: Properties) -> Result<(Msg, GraphId)> {
        self.create_graph_with_id(indradb::util::generate_uuid_v1(), properties)
            .await
    }

    async fn create_graph_with_id(
        &self,
        graph_id: GraphId,
        properties: Properties,
    ) -> Result<(Msg, GraphId)>;

    async fn list_graphs(&self) -> Result<Vec<Node>>;

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph>;

    async fn create_node(&self, args: (GraphId, Properties)) -> Result<(Msg, NodeId)>;

    async fn read_node(&self, node_id: NodeId) -> Result<Node>;

    async fn update_node(&self, args: (NodeId, Properties), graph_id: GraphId) -> Result<Msg>;

    async fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId) -> Result<Msg>;

    async fn recreate_edge(&self, edge: Edge, properties: Properties) -> Result<()>;

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg>;

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Msg, EdgeId)>;

    async fn read_edge_properties(&self, msg: Edge) -> Result<Properties>;

    async fn update_edge(&self, args: (Edge, Properties), graph_id: GraphId) -> Result<Msg>;

    async fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Msg>;
}
