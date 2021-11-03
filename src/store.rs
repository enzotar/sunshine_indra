use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, RangeVertexQuery, RocksdbDatastore, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, Vertex, VertexPropertyQuery, VertexQuery,
    VertexQueryExt,
};
use serde::__private::de::InternallyTaggedUnitVisitor;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::msg::{
    Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId, Query, Reply,
};

use crate::error::{Error, Result};

const VERTEX_PROPERTY_HOLDER: &str = "data";
const VERTEX_TYPE: &str = "node";

const GRAPH_ROOT_TYPE: &str = "_root_type";
const STATE_ID_PROPERTY: &str = "_state_id_prop";

pub struct Store {
    datastore: RocksdbDatastore,
    root_node_type: Type,
    undo: Vec<Msg>,
    history: Vec<Msg>,
}

pub struct CloudState {
    msgs: Vec<Msg>,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore =
            RocksdbDatastore::new(&cfg.db_path, None).map_err(Error::DatastoreCreate)?;
        let store = Store {
            datastore,
            root_node_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
            undo: vec![],
            history: vec![], //todo
        };
        Ok(store)
        //         return Reply::Error(e.to_string());
    }

    pub fn execute(&mut self, msg: Msg) -> Result<Reply> {
        self.execute_impl(msg, true)
    }

    pub fn execute_impl(&mut self, msg: Msg, modify_undo: bool) -> Result<Reply> {
        let (undo_msg, reply) = match msg.clone() {
            Msg::CreateGraph(properties) => self
                .create_graph(properties)
                .map(|(undo_msg, node)| (Some(undo_msg), Reply::Id(node)))?,
            Msg::MutateState(mutate_state) => self
                .execute_mutate_state(mutate_state)
                .map(|(undo_msg, reply)| (Some(undo_msg), reply))?,
            Msg::Query(read_only) => (None, self.execute_read_only(read_only)?),
            Msg::DeleteGraph(_) => todo!(),
            // Msg::Undo => self
            //     .execute_impl(self.undo.pop().unwrap(), false)
            //     .map(|reply| (None, reply)),
        };

        if modify_undo {
            if let Some(undo_msg) = undo_msg {
                self.undo.push(undo_msg);
            }
        }

        self.history.push(msg);

        Ok(reply)
    }

    fn execute_mutate_state(&self, msg: MutateState) -> Result<(Msg, Reply)> {
        let MutateState { kind, graph_id } = msg;

        let (undo_msg, reply) = match kind {
            MutateStateKind::CreateNode(properties) => self
                .create_node((Some(graph_id), properties))
                .map(|(undo_msg, node_id)| (undo_msg, Reply::Id(node_id)))?,
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
            Query::ReadEdge(msg) => self.read_edge(msg).map(Reply::Edge),
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

        let (undo_msg, node_id) = self.create_node((None, properties))?; // todo

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

    fn create_node(
        &self,
        (graph_id, properties): (Option<GraphId>, JsonValue),
    ) -> Result<(Msg, NodeId)> {
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

        if let Some(graph_id) = graph_id {
            let edge_key = EdgeKey {
                outbound_id: graph_id,
                inbound_id: node.id,
                t: Type(indradb::util::generate_uuid_v1().to_string()),
            };
            if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
                return Err(Error::CreateEdgeFailed);
            }
        }

        Ok((
            Msg::MutateState(MutateState {
                graph_id: graph_id.unwrap(),
                kind: MutateStateKind::DeleteNode(node.id),
            }),
            node.id,
        ))
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
            .map(|edge| self.read_edge(Edge::try_from(edge.key)?))
            .collect::<Result<Vec<_>>>()?;

        let inbound_edges = trans
            .get_edges(inbound_query)
            .map_err(Error::GetEdgesOfNodes)?
            .into_iter()
            .map(|edge| self.read_edge(Edge::try_from(edge.key)?))
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

    // fn recreate_node(&self, node: Node) -> Result<Msg> {
    //     let trans = self.transaction()?;

    //     let node_type = Type::new(VERTEX_TYPE).map_err(Error::CreateType)?;
    //     let node: Vertex = Vertex::with_id(node.node_id, node_type);
    //     trans.create_vertex(&node).map_err(Error::CreateNode)?;

    //     let vertex_query = SpecificVertexQuery::single(node.id).into();

    //     let vertex_property_query = VertexPropertyQuery {
    //         inner: vertex_query,
    //         name: VERTEX_PROPERTY_HOLDER.into(),
    //     };
    //     trans
    //         .set_vertex_properties(vertex_property_query, &properties)
    //         .map_err(Error::SetNodeProperties)?;

    //     let edge_key = EdgeKey {
    //         outbound_id: graph_id,
    //         inbound_id: node.id,
    //         t: Type(indradb::util::generate_uuid_v1().to_string()),
    //     };
    //     if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
    //         return Err(Error::CreateEdgeFailed);
    //     }

    //     Ok((
    //         Msg::MutateState(MutateState {
    //             graph_id,
    //             kind: MutateStateKind::DeleteNode(node.id),
    //         }),
    //         node.id,
    //     ))
    // }

    // deletes inbound and outbound edges as well
    fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;
        let query = SpecificVertexQuery { ids: vec![node_id] };

        let prev_state = self.read_node(node_id)?;

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

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::CreateNode(prev_state.properties),
        }))
    }

    fn create_edge(&self, msg: Edge, graph_id: GraphId) -> Result<(Msg, EdgeId)> {
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
                kind: MutateStateKind::DeleteEdge(Edge::try_from(edge_key).unwrap()), // todo
            }),
            edge_id,
        ))
    }

    fn read_edge(&self, msg: Edge) -> Result<Edge> {
        let trans = self.transaction()?;
        let edge_key: EdgeKey = msg.into();
        let query = SpecificEdgeQuery {
            keys: vec![edge_key.clone()],
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

        Ok(Edge {
            id: indradb::util::generate_uuid_v1(),
            from: edge_key.outbound_id,
            to: edge_key.inbound_id,
            properties,
        })
    }

    fn update_edge(&self, (edge, properties): (Edge, JsonValue), graph_id: GraphId) -> Result<Msg> {
        let prev_state = self.read_node(edge.id)?;

        let trans = self.transaction()?;
        let edge_key = edge.clone().into();

        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };

        trans
            .set_edge_properties(query, &edge.properties)
            .map_err(Error::UpdateEdgeProperties)?;

        Ok(Msg::MutateState(MutateState {
            graph_id,
            kind: MutateStateKind::UpdateEdge((edge, prev_state.properties)),
        }))
    }

    fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Msg> {
        let trans = self.transaction()?;
        let edge_key = edge.clone().into();
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        trans.delete_edges(query).map_err(Error::DeleteEdge)?;
        Ok(Msg::MutateState(MutateState {
            kind: MutateStateKind::CreateEdge(edge),
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
            .into_node()
            .unwrap()
            .node_id;

        dbg!(graph_id.clone());

        let make_msg_mut = |kind: MutateStateKind| {
            Msg::MutateState(MutateState {
                kind,
                graph_id: graph_id.clone(),
            })
        };

        let print_state = || {
            let reply = store.execute(Msg::Query(Query::ReadGraph(graph_id.clone())));
            dbg!(reply);
        };

        let create_node = |properties: serde_json::Value| {
            store
                .execute(make_msg_mut(MutateStateKind::CreateNode(properties)))
                .unwrap()
                .into_node()
                .unwrap()
                .node_id
        };

        let id1 = create_node(serde_json::json!({
            "name": "first_vertex",
        }));

        dbg!(id1.clone());

        print_state();

        print_state();

        // store
        //     .execute(make_msg_mut(MutateStateKind::UpdateNode((
        //         id1,
        //         serde_json::json!({
        //             "name": "updated_first_vertex",
        //         }),
        //     ))))
        //     .unwrap();

        // print_state();

        // let id2 = create_node(serde_json::json!({
        //     "name": "second_vertex",
        // }));

        // store
        //     .execute(make_msg_mut(MutateStateKind::CreateEdge(Edge {
        //         from: id1.clone(),
        //         to: id2.clone(),
        //         properties: serde_json::json!({
        //             "name": "first_edge",
        //         }),
        //         id: indradb::util::generate_uuid_v1(),
        //     })))
        //     .unwrap();

        // store
        //     .execute(make_msg_mut(MutateStateKind::CreateEdge(Edge {
        //         from: id1.clone(),
        //         to: id2.clone(),
        //         properties: serde_json::json!({
        //             "name": "second_edge",
        //         }),
        //         id: indradb::util::generate_uuid_v1(),
        //     })))
        //     .unwrap();

        // print_state();

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
