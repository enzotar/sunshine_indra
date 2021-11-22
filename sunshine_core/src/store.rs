use async_trait::async_trait;
use uuid::Uuid;

pub use crate::error::{Error, Result};

use crate::msg::{
    Action, CreateEdge, Edge, EdgeId, Graph, GraphId, MutateKind, Node, NodeId, Properties,
    QueryKind, RecreateNode, Reply,
};

#[derive(Debug)]
pub enum Operation {
    Undo,
    Redo,
    Other,
}

#[async_trait]
pub trait Datastore: Send + Sync {
    fn undo_buf(&mut self) -> &mut Vec<Action>;

    fn redo_buf(&mut self) -> &mut Vec<Action>;

    fn history_buf(&mut self) -> &mut Vec<Action>;

    async fn execute(&mut self, msg: Action) -> Result<Reply> {
        self.execute_impl(msg, Operation::Other).await
    }

    async fn execute_impl(&mut self, msg: Action, operation: Operation) -> Result<Reply> {
        let (reverse_msg, reply) = match msg.clone() {
            Action::CreateGraph(properties) => self
                .create_graph(properties)
                .await
                .map(|(reverse_msg, node)| (Some(reverse_msg), Reply::Id(node)))?,
            Action::CreateGraphWithId(uuid, properties) => self
                .create_graph_with_id(uuid, properties)
                .await
                .map(|(reverse_msg, node)| (Some(reverse_msg), Reply::Id(node)))?,
            Action::Mutate(uuid, mutate_state) => self
                .execute_mutate_state((uuid, mutate_state))
                .await
                .map(|(reverse_msg, reply)| (Some(reverse_msg), reply))?,
            Action::Query(read_only) => (None, self.execute_read_only(read_only).await?),
            Action::DeleteGraph(_) => todo!(),
            Action::Undo => {
                let reverse_msg = self.undo_buf().pop().ok_or(Error::UndoBufferEmpty)?;
                self.execute_impl(reverse_msg, Operation::Undo)
                    .await
                    .map(|reply| (None, reply))?
            }
            Action::Redo => {
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

    async fn execute_mutate_state(&self, msg: (Uuid, MutateKind)) -> Result<(Action, Reply)> {
        // let MutateState { kind, graph_id } = msg;
        let (graph_id, kind) = msg;

        let (undo_msg, reply) = match kind {
            MutateKind::CreateNode(properties) => self
                .create_node((graph_id, properties))
                .await
                .map(|(undo_msg, node_id)| (undo_msg, Reply::Id(node_id)))?,
            MutateKind::CreateNodeWithId((node_id, properties)) => self
                .create_node_with_id(node_id, (graph_id, properties))
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateKind::RecreateNode(recreate_node) => {
                let node_id = recreate_node.node_id;
                self.recreate_node(recreate_node, graph_id)
                    .await
                    .map(|undo_msg| (undo_msg, Reply::Id(node_id)))?
            }
            MutateKind::UpdateNode((node_id, properties)) => self
                .update_node((node_id, properties), graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateKind::DeleteNode(node_id) => self
                .delete_node(node_id, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateKind::CreateEdge(edge) => self
                .create_edge(edge, graph_id)
                .await
                .map(|(undo_msg, edge_id)| (undo_msg, Reply::Id(edge_id)))?,
            MutateKind::UpdateEdge(edge) => self
                .update_edge(edge, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateKind::DeleteEdge(edge) => self
                .delete_edge(edge, graph_id)
                .await
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
        };

        self.update_state_id(graph_id).await?;

        Ok((undo_msg, reply))
    }

    async fn execute_read_only(&self, msg: QueryKind) -> Result<Reply> {
        match msg {
            QueryKind::ReadEdgeProperties(msg) => {
                self.read_edge_properties(msg).await.map(Reply::Properties)
            }
            QueryKind::ReadNode(msg) => self.read_node(msg).await.map(Reply::Node),
            QueryKind::ReadGraph(read_graph) => self.read_graph(read_graph).await.map(Reply::Graph),
            QueryKind::ListGraphs => self.list_graphs().await.map(Reply::NodeList),
        }
    }

    async fn update_state_id(&self, graph_id: GraphId) -> Result<()>;

    async fn create_graph(&self, properties: Properties) -> Result<(Action, GraphId)> {
        self.create_graph_with_id(indradb::util::generate_uuid_v1(), properties)
            .await
    }

    async fn create_graph_with_id(
        &self,
        graph_id: GraphId,
        properties: Properties,
    ) -> Result<(Action, GraphId)>;

    async fn list_graphs(&self) -> Result<Vec<(NodeId, Properties)>>;

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph>;

    async fn create_node(&self, args: (GraphId, Properties)) -> Result<(Action, NodeId)> {
        let node_id = indradb::util::generate_uuid_v1();

        self.create_node_with_id(node_id, args)
            .await
            .map(|msg| (msg, node_id))
    }

    async fn create_node_with_id(
        &self,
        node_id: NodeId,
        (graph_id, properties): (GraphId, Properties),
    ) -> Result<Action>;

    async fn read_node(&self, node_id: NodeId) -> Result<Node>;

    async fn update_node(&self, args: (NodeId, Properties), graph_id: GraphId) -> Result<Action>;

    async fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId)
        -> Result<Action>;

    async fn recreate_edge(&self, edge: Edge, properties: Properties) -> Result<()>;

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Action>;

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Action, EdgeId)>;

    async fn read_edge_properties(&self, msg: Edge) -> Result<Properties>;

    async fn update_edge(&self, args: (Edge, Properties), graph_id: GraphId) -> Result<Action>;

    async fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Action>;
}
