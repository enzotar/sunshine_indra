use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, RangeVertexQuery, RocksdbDatastore, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, Vertex, VertexPropertyQuery, VertexQuery,
    VertexQueryExt,
};

use serde_json::Value as JsonValue;
use uuid::Uuid;

use sunshine_core::msg::{
    CreateEdge, Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId,
    Query, RecreateNode, Reply,
};

use crate::error::{Error, Result};

const VERTEX_PROPERTY_HOLDER: &str = "data";
const VERTEX_TYPE: &str = "node";

const GRAPH_ROOT_TYPE: &str = "_root_type";
const STATE_ID_PROPERTY: &str = "_state_id_prop";

// cloud Dgraph

// local // msg

// ui

impl sunshine_core::Store for Store {
    type Error = Error;

    fn execute(&mut self, msg: Msg) -> Result<Reply> {
        self.execute_impl(msg, Operation::Other)
    }
}

pub struct Store {
    datastore: RocksdbDatastore,
    root_node_type: Type,
    undo: Vec<Msg>,
    redo: Vec<Msg>,
    history: Vec<Msg>,
}

#[derive(Debug)]
pub enum Operation {
    Undo,
    Redo,
    Other,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore =
            RocksdbDatastore::new(&cfg.db_path, None).map_err(Error::DatastoreCreate)?;
        let store = Store {
            datastore,
            root_node_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
            undo: Vec::new(),
            redo: Vec::new(),
            history: Vec::new(),
        };
        Ok(store)
    }

    // create node
    // undo

    fn execute_impl(&mut self, msg: Msg, operation: Operation) -> Result<Reply> {
        let (reverse_msg, reply) = match msg.clone() {
            Msg::CreateGraph(properties) => self
                .create_graph(properties)
                .map(|(reverse_msg, node)| (Some(reverse_msg), Reply::Id(node)))?,
            Msg::MutateState(mutate_state) => self
                .execute_mutate_state(mutate_state)
                .map(|(reverse_msg, reply)| (Some(reverse_msg), reply))?,
            Msg::Query(read_only) => (None, self.execute_read_only(read_only)?),
            Msg::DeleteGraph(_) => todo!(),
            Msg::Undo => {
                let reverse_msg = self.undo.pop().ok_or(Error::UndoBufferEmpty)?;
                self.execute_impl(reverse_msg, Operation::Undo)
                    .map(|reply| (None, reply))?
            }
            Msg::Redo => {
                let reverse_msg = self.redo.pop().ok_or(Error::RedoBufferEmpty)?;
                self.execute_impl(reverse_msg, Operation::Redo)
                    .map(|reply| (None, reply))?
            }
        };

        if let Some(reverse_msg) = reverse_msg {
            match operation {
                Operation::Other => {
                    self.redo.clear();
                    self.undo.push(reverse_msg);
                }
                Operation::Redo => self.undo.push(reverse_msg),
                Operation::Undo => self.redo.push(reverse_msg),
            }
        }

        self.history.push(msg);

        Ok(reply)
    }

    fn execute_mutate_state(&self, msg: MutateState) -> Result<(Msg, Reply)> {
        let MutateState { kind, graph_id } = msg;

        let (undo_msg, reply) = match kind {
            MutateStateKind::CreateNode(properties) => self
                .create_node((graph_id, properties))
                .map(|(undo_msg, node_id)| (undo_msg, Reply::Id(node_id)))?,
            MutateStateKind::RecreateNode(recreate_node) => {
                let node_id = recreate_node.node_id;
                self.recreate_node(recreate_node, graph_id)
                    .map(|undo_msg| (undo_msg, Reply::Id(node_id)))?
            }
            MutateStateKind::UpdateNode((node_id, properties)) => self
                .update_node((node_id, properties), graph_id)
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::DeleteNode(node_id) => self
                .delete_node(node_id, graph_id)
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::CreateEdge(edge) => self
                .create_edge(edge, graph_id)
                .map(|(undo_msg, edge_id)| (undo_msg, Reply::Id(edge_id)))?,
            MutateStateKind::UpdateEdge(edge) => self
                .update_edge(edge, graph_id)
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
            MutateStateKind::DeleteEdge(edge) => self
                .delete_edge(edge, graph_id)
                .map(|undo_msg| (undo_msg, Reply::Empty))?,
        };

        self.update_state_id(graph_id)?;

        Ok((undo_msg, reply))
    }

    fn execute_read_only(&self, msg: Query) -> Result<Reply> {
        match msg {
            Query::ReadEdgeProperties(msg) => self.read_edge_properties(msg).map(Reply::Json),
            Query::ReadNode(msg) => self.read_node(msg).map(Reply::Node),
            Query::ReadGraph(read_graph) => self.read_graph(read_graph).map(Reply::Graph),
            Query::ListGraphs => self.list_graphs().map(Reply::NodeList),
        }
    }

    fn update_state_id(&self, graph_id: Uuid) -> Result<()> {
        let mut graph_root = self.read_node(graph_id)?;
        let properties = graph_root.properties.as_object_mut().unwrap();
        let current_id = properties.get(STATE_ID_PROPERTY).unwrap().as_u64().unwrap();
        let new_id = JsonValue::Number(serde_json::Number::from(current_id + 1));

        properties.insert(STATE_ID_PROPERTY.into(), new_id);

        self.update_node((graph_id, graph_root.properties), graph_id)?;

        Ok(())
    }

    fn create_graph(&self, properties: JsonValue) -> Result<(Msg, GraphId)> {
        let mut properties = properties;
        let state_id = JsonValue::Number(serde_json::Number::from(0u64));
        properties
            .as_object_mut()
            .unwrap()
            .insert(STATE_ID_PROPERTY.into(), state_id);

        let node_id = self.create_graph_root(properties)?;

        Ok((Msg::DeleteGraph(node_id), node_id))
    }

    fn list_graphs(&self) -> Result<Vec<Node>> {
        let trans = self.transaction()?;
        trans
            .get_vertices(RangeVertexQuery {
                limit: 0,
                t: Some(self.root_node_type.clone()),
                start_id: None,
            })
            .map_err(Error::GetNodes)?
            .iter()
            .map(|node| self.read_node(node.id))
            .collect::<Result<Vec<_>>>()
    }

    fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let graph_node = self.read_node(graph_id)?;
        let nodes = graph_node
            .outbound_edges
            .iter()
            .map(|edge| self.read_node(edge.to))
            .collect::<Result<Vec<_>>>()?;
        let state_id = graph_node
            .properties
            .as_object()
            .unwrap()
            .get(STATE_ID_PROPERTY)
            .unwrap()
            .as_u64()
            .unwrap();

        Ok(Graph { nodes, state_id })
    }

    fn create_node(&self, (graph_id, properties): (GraphId, JsonValue)) -> Result<(Msg, NodeId)> {
        let trans = self.transaction()?;

        let node_type = Type::new(VERTEX_TYPE).map_err(Error::CreateType)?;
        let node: Vertex = Vertex::new(node_type);
        trans.create_vertex(&node).map_err(Error::CreateNode)?;

        let vertex_query = SpecificVertexQuery::single(node.id).into();

        let vertex_property_query = VertexPropertyQuery {
            inner: vertex_query,
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_vertex_properties(vertex_property_query, &properties)
            .map_err(Error::SetNodeProperties)?;

        let edge_key = EdgeKey {
            outbound_id: graph_id,
            inbound_id: node.id,
            t: Type(indradb::util::generate_uuid_v1().to_string()),
        };
        if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
            return Err(Error::CreateEdgeFailed);
        }

        Ok((
            Msg::MutateState(MutateState {
                graph_id,
                kind: MutateStateKind::DeleteNode(node.id),
            }),
            node.id,
        ))
    }

    fn create_graph_root(&self, properties: JsonValue) -> Result<NodeId> {
        let trans = self.transaction()?;

        let node_type = Type::new(GRAPH_ROOT_TYPE).map_err(Error::CreateType)?;
        let node: Vertex = Vertex::new(node_type);
        trans.create_vertex(&node).map_err(Error::CreateNode)?;

        let vertex_query = SpecificVertexQuery::single(node.id).into();

        let vertex_property_query = VertexPropertyQuery {
            inner: vertex_query,
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_vertex_properties(vertex_property_query, &properties)
            .map_err(Error::SetNodeProperties)?;

        Ok(node.id)
    }

    fn read_node(&self, node_id: NodeId) -> Result<Node> {
        let trans = self.transaction()?;
        // let uuid = node_id;

        let query = SpecificVertexQuery::single(node_id);

        // let vertex_query: VertexQuery = query.clone().into();

        let outbound_query = query.clone().outbound();

        let inbound_query = query.clone().inbound();

        let mut properties = trans
            .get_all_vertex_properties(VertexQuery::Specific(query))
            .map_err(Error::GetNodes)?;

        let properties = match properties.len() {
            1 => properties.pop().unwrap().props.pop().unwrap().value,
            _ => unreachable!(),
        };

        let outbound_edges = trans
            .get_edges(outbound_query)
            .map_err(Error::GetEdgesOfNodes)?
            .into_iter()
            .map(|edge| Edge::try_from(edge.key).map_err(Error::InvalidId))
            .collect::<Result<Vec<_>>>()?;

        let inbound_edges = trans
            .get_edges(inbound_query)
            .map_err(Error::GetEdgesOfNodes)?
            .into_iter()
            .map(|edge| Edge::try_from(edge.key).map_err(Error::InvalidId))
            .collect::<Result<Vec<_>>>()?;

        let node = Node {
            node_id,
            outbound_edges,
            inbound_edges,
            properties,
        };

        Ok(node)
    }

    fn update_node(&self, (node_id, value): (NodeId, JsonValue), graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;

        let query = SpecificVertexQuery { ids: vec![node_id] };

        let prev_state = self.read_node(node_id)?;

        trans
            .set_vertex_properties(
                VertexPropertyQuery {
                    inner: query.into(),
                    name: VERTEX_PROPERTY_HOLDER.into(),
                },
                &value,
            )
            .map_err(Error::UpdateNode)?;

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::UpdateNode((node_id, prev_state.properties)),
        }))
    }

    fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;

        let node_type = Type::new(VERTEX_TYPE).map_err(Error::CreateType)?;
        let node: Vertex = Vertex::with_id(recreate_node.node_id, node_type);
        trans.create_vertex(&node).map_err(Error::CreateNode)?;

        let vertex_query = SpecificVertexQuery::single(recreate_node.node_id).into();

        let vertex_property_query = VertexPropertyQuery {
            inner: vertex_query,
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_vertex_properties(vertex_property_query, &recreate_node.properties)
            .map_err(Error::SetNodeProperties)?;

        recreate_node
            .edges
            .into_iter()
            .try_for_each(|(edge, props)| self.recreate_edge(edge, props))?;

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::DeleteNode(recreate_node.node_id),
        }))
    }

    fn recreate_edge(&self, edge: Edge, properties: JsonValue) -> Result<()> {
        let trans = self.transaction()?;
        let edge_key = edge.into();
        if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
            return Err(Error::CreateEdgeFailed);
        }
        let get_created_edge = SpecificEdgeQuery::single(edge_key);
        let query = EdgePropertyQuery {
            inner: get_created_edge.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_edge_properties(query, &properties)
            .map_err(Error::SetEdgeProperties)?;

        Ok(())
    }

    // deletes inbound and outbound edges as well
    fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;
        let query = SpecificVertexQuery { ids: vec![node_id] };

        let deleted_node = self.read_node(node_id)?;

        let outbound_query = query.clone().outbound();
        let inbound_query = query.clone().inbound();
        trans
            .delete_edges(outbound_query)
            .map_err(Error::DeleteOutboundEdges)?;
        trans
            .delete_edges(inbound_query)
            .map_err(Error::DeleteInboundEdges)?;
        trans
            .delete_vertices(VertexQuery::Specific(query))
            .map_err(Error::DeleteNode)?;

        let edges = deleted_node
            .inbound_edges
            .into_iter()
            .chain(deleted_node.outbound_edges.into_iter())
            .map(|edge| self.read_edge_properties(edge).map(|props| (edge, props)))
            .collect::<Result<Vec<_>>>()?;

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::RecreateNode(RecreateNode {
                node_id,
                properties: deleted_node.properties,
                edges,
            }),
        }))
    }

    fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Msg, EdgeId)> {
        let trans = self.transaction()?;
        let edge_id = indradb::util::generate_uuid_v1();
        let edge_type = Type::new(edge_id.to_string()).map_err(Error::CreateType)?;
        let edge_key = EdgeKey {
            outbound_id: msg.from,
            inbound_id: msg.to,
            t: edge_type,
        };
        if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
            return Err(Error::CreateEdgeFailed);
        }
        let get_created_edge = SpecificEdgeQuery::single(edge_key.clone());
        let query = EdgePropertyQuery {
            inner: get_created_edge.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_edge_properties(query, &msg.properties)
            .map_err(Error::SetEdgeProperties)?;

        Ok((
            Msg::MutateState(MutateState {
                graph_id,
                kind: MutateStateKind::DeleteEdge(Edge::try_from(edge_key).unwrap()),
            }),
            edge_id,
        ))
    }

    fn read_edge_properties(&self, msg: Edge) -> Result<JsonValue> {
        let trans = self.transaction()?;
        let edge_key: EdgeKey = msg.into();
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        let mut properties = trans
            .get_edge_properties(query)
            .map_err(Error::GetEdgeProperties)?;

        let properties = match properties.len() {
            1 => properties.pop().unwrap().value,
            0 => JsonValue::Null,
            _ => unreachable!(),
        };

        Ok(properties)
    }

    fn update_edge(&self, (edge, properties): (Edge, JsonValue), graph_id: GraphId) -> Result<Msg> {
        let prev_state = self.read_node(edge.id)?;

        let trans = self.transaction()?;
        let edge_key = edge.into();

        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };

        trans
            .set_edge_properties(query, &properties)
            .map_err(Error::UpdateEdgeProperties)?;

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::UpdateEdge((edge, prev_state.properties)),
        }))
    }

    fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;
        let edge_key = edge.into();
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        trans.delete_edges(query).map_err(Error::DeleteEdge)?;
        let properties = self.read_edge_properties(edge)?;
        Ok(Msg::MutateState(MutateState {
            kind: MutateStateKind::CreateEdge(CreateEdge {
                to: edge.to,
                from: edge.from,
                properties,
            }),
            graph_id,
        }))
    }

    fn transaction(&self) -> Result<impl Transaction> {
        self.datastore
            .transaction()
            .map_err(Error::CreateTransaction)
    }
}

pub struct Config {
    pub db_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let cfg = Config {
            db_path: "newdb".into(),
        };
        let mut store = Store::new(&cfg).unwrap();

        let graph_id = store
            .execute(Msg::CreateGraph(serde_json::json!({
                "name": "first_graph",
            })))
            .unwrap()
            .as_id()
            .unwrap();

        // dbg!(graph_id);

        let make_msg_mut = |kind: MutateStateKind| Msg::MutateState(MutateState { kind, graph_id });

        let print_state = |store: &mut Store| {
            let reply = store
                .execute(Msg::Query(Query::ReadGraph(graph_id)))
                .unwrap();
            dbg!(&store.undo);
            dbg!(&store.redo);
            dbg!(reply);
        };

        let create_node = |store: &mut Store, properties: serde_json::Value| {
            store
                .execute(make_msg_mut(MutateStateKind::CreateNode(properties)))
                .unwrap()
                .as_id()
                .unwrap()
        };

        ///
        let id1 = create_node(
            &mut store,
            serde_json::json!({
                "name": "first_vertex",
            }),
        );

        ///
        store
            .execute(make_msg_mut(MutateStateKind::UpdateNode((
                id1,
                serde_json::json!({
                    "name": "updated_first_vertex",
                }),
            ))))
            .unwrap();

        ///
        let id2 = create_node(
            &mut store,
            serde_json::json!({
                "name": "second_vertex",
            }),
        );

        ///
        store
            .execute(make_msg_mut(MutateStateKind::CreateEdge(CreateEdge {
                from: id1,
                to: id2,
                properties: serde_json::json!({
                    "name": "first_edge",
                }),
            })))
            .unwrap();

        ///
        store
            .execute(make_msg_mut(MutateStateKind::CreateEdge(CreateEdge {
                from: id1,
                to: id2,
                properties: serde_json::json!({
                    "name": "second_edge",
                }),
            })))
            .unwrap();

        print_state(&mut store);

        store.execute(Msg::Undo).unwrap();
        store.execute(Msg::Undo).unwrap();
        store.execute(Msg::Undo).unwrap();
        store.execute(Msg::Undo).unwrap();
        store.execute(Msg::Undo).unwrap();

        print_state(&mut store);

        store.execute(Msg::Redo).unwrap();
        store.execute(Msg::Redo).unwrap();

        store.execute(Msg::Undo).unwrap();

        print_state(&mut store);

        /*

        let reply = store.execute(Msg::CreateEdge(CreateEdge {
            directed: false,
            from: id1.clone(),
            edge_type: "edge_type1".into(),
            to: id2,
            properties: serde_json::json!({
                "name": "first_edge",
            }),
        }));

        println!("{:#?}", reply);

        let reply = store.execute(Msg::ReadVertex(id1.clone()));

        println!("{:#?}", reply);

        let read = store.read_vertex(&id1);
        //dbg! {read};

        let get_all = store.get_all_nodes_and_edges();
        //dbg! {get_all};
        */
    }
}
/*
fn map_reply_tuple<T, F: Fn(T) -> Reply>(
    res: Result<(Msg, T)>,
    reply_fn: F,
) -> Result<(Msg, Reply)> {
    match res {
        Ok((msg, reply)) => Ok((msg, reply_fn(reply))),
        Err(e) => Err(e),
    }
}
*/
