use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, MemoryDatastore, MemoryTransaction, RangeVertexQuery,
    SpecificEdgeQuery, SpecificVertexQuery, Transaction, Type, Vertex, VertexPropertyQuery,
    VertexQuery, VertexQueryExt,
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

use crate::msg::{
    Edge, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId, Query, Reply,
};

use crate::error::{Error, Result};

const VERTEX_PROPERTY_HOLDER: &str = "data";
const VERTEX_TYPE: &str = "node";

const EDGE_TYPE: &str = "edge";

const GRAPH_ROOT_TYPE: &str = "_root_type";
const GRAPH_ROOT_EDGE_TYPE: &str = "_root_edge_type";
const STATE_ID_PROPERTY: &str = "_state_id_prop";

#[derive(Debug)]
pub struct Store {
    datastore: MemoryDatastore,
    root_node_type: Type,
    root_edge_type: Type,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore = create_db(&cfg.db_path).map_err(Error::DatastoreCreate)?;
        let store = Store {
            datastore: datastore,
            root_node_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
            root_edge_type: Type::new(GRAPH_ROOT_EDGE_TYPE).unwrap(),
        };
        Ok(store)
        //         return Reply::Error(e.to_string());
    }

    pub fn execute(&self, msg: Msg) -> Reply {
        match msg {
            Msg::CreateGraph(properties) => Reply::from_node(self.create_graph(properties)),
            Msg::MutateState(mutate_state) => self.execute_mutate_state(mutate_state),
            Msg::Query(read_only) => self.execute_read_only(read_only),
        }
    }

    fn execute_mutate_state(&self, msg: MutateState) -> Reply {
        let MutateState { kind, graph_id } = msg;

        let reply = match kind {
            MutateStateKind::CreateNode(node_args) => {
                Reply::from_node(self.create_node((Some(graph_id), node_args)))
            }
            MutateStateKind::UpdateNode((node_id, properties)) => {
                Reply::from_empty(self.update_node((node_id, properties)))
            }
            MutateStateKind::DeleteNode(msg) => Reply::from_empty(self.delete_node(msg.node_id)),
            MutateStateKind::CreateEdge(msg) => Reply::from_edge(self.create_edge(msg)),
            MutateStateKind::UpdateEdge(msg) => Reply::from_empty(self.update_edge(msg)),
            MutateStateKind::DeleteEdge(msg) => Reply::from_empty(self.delete_edge(msg)),
            MutateStateKind::DeleteGraph(_) => todo!(),
            MutateStateKind::ReverseEdge(_) => todo!(),
        };

        if let Reply::Error(e) = reply {
            return Reply::Error(e);
        }

        if let Err(e) = self.update_state_id(graph_id) {
            return Reply::Error(e.to_string());
        }

        reply
    }

    fn execute_read_only(&self, msg: Query) -> Reply {
        match msg {
            Query::ReadEdge(msg) => Reply::from_edge(self.read_edge(msg)),
            Query::ReadNode(msg) => Reply::from_node(self.read_vertex(msg)),
            Query::ReadGraph(read_graph) => Reply::from_graph(self.read_graph(read_graph)),
            Query::ListGraphs => Reply::from_node_list(self.list_graphs()),
            _ => todo!(),
        }
    }

    fn update_state_id(&self, graph_id: Uuid) -> Result<()> {
        let mut graph_root = self.read_vertex(graph_id)?;
        let mut properties = graph_root.properties.as_object_mut().unwrap();
        let current_id = properties.get(STATE_ID_PROPERTY).unwrap().as_u64().unwrap();
        let new_id = JsonValue::Number(serde_json::Number::from(current_id + 1));

        properties.insert(STATE_ID_PROPERTY.into(), new_id);

        self.update_node((graph_id, graph_root.properties))?;

        Ok(())
    }

    fn create_graph(&self, properties: JsonValue) -> Result<Node> {
        let mut properties = properties;
        let state_id = JsonValue::Number(serde_json::Number::from(0u64));
        properties
            .as_object_mut()
            .unwrap()
            .insert(STATE_ID_PROPERTY.into(), state_id);

        self.create_node((None, Node::from_properties(properties)))
    }

    fn list_graphs(&self) -> Result<Vec<Node>> {
        let trans = self.transaction()?;
        trans
            .get_vertices(RangeVertexQuery {
                limit: 0,
                t: Some(self.root_node_type.clone()),
                start_id: None,
            })
            .map_err(Error::GetVertices)?
            .iter()
            .map(|vertex| self.read_vertex(vertex.id))
            .collect::<Result<Vec<_>>>()
    }

    fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let graph_node = self.read_vertex(graph_id)?;
        let vertices = graph_node
            .outbound_edges
            .unwrap()
            .iter()
            .map(|edge| self.read_vertex(edge.to))
            .collect::<Result<Vec<_>>>()?;
        let state_id = graph_node
            .properties
            .as_object()
            .unwrap()
            .get(STATE_ID_PROPERTY)
            .unwrap()
            .as_u64()
            .unwrap()
            .into();

        Ok(Graph { vertices, state_id })
    }

    // does user have latest state?

    fn create_node(&self, (graph_id, node_args): (Option<GraphId>, Node)) -> Result<Node> {
        let trans = self.transaction()?;

        let vertex_type = Type::new(VERTEX_TYPE).map_err(Error::CreateType)?;
        let vertex: Vertex = Vertex::new(vertex_type);
        trans.create_vertex(&vertex).map_err(Error::CreateVertex)?;

        let vertex_query = SpecificVertexQuery::single(vertex.id).into();

        let vertex_property_query = VertexPropertyQuery {
            inner: vertex_query,
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_vertex_properties(vertex_property_query, &node_args.properties)
            .map_err(Error::SetVertexProperties)?;

        if let Some(graph_id) = graph_id {
            let edge_key = EdgeKey {
                outbound_id: graph_id.clone(),
                inbound_id: vertex.id,
                t: self.root_edge_type.clone(),
            };
            dbg!(edge_key.clone());
            if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
                return Err(Error::CreateEdgeFailed);
            }
        }
        Ok(Node {
            node_id: vertex.id,
            properties: node_args.properties,
            outbound_edges: None,
            inbound_edges: None,
        })
    }

    fn read_vertex(&self, node_id: NodeId) -> Result<Node> {
        let trans = self.transaction()?;
        // let uuid = node_id;

        let query = SpecificVertexQuery::single(node_id);

        // let vertex_query: VertexQuery = query.clone().into();

        let outbound_query = query.clone().outbound();

        let inbound_query = query.clone().inbound();

        let mut properties = trans
            .get_all_vertex_properties(VertexQuery::Specific(query))
            .map_err(Error::GetVertices)?;
        assert_eq!(properties.len(), 1);

        let properties = properties.pop().unwrap().props.pop().unwrap().value;

        let outbound_edges = Some(
            trans
                .get_edges(outbound_query)
                .map_err(Error::GetEdgesOfVertex)?
                .into_iter()
                .map(|edge| Edge::from(edge.key))
                .collect(),
        );

        let inbound_edges = Some(
            trans
                .get_edges(inbound_query)
                .map_err(Error::GetEdgesOfVertex)?
                .into_iter()
                .map(|edge| Edge::from(edge.key))
                .collect(),
        );
        let node = Node {
            node_id,
            outbound_edges,
            inbound_edges,
            properties,
        };
        dbg!(node.clone());

        Ok(node)
    }

    fn update_node(&self, (node_id, value): (NodeId, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;

        let query = SpecificVertexQuery { ids: vec![node_id] };
        trans
            .set_vertex_properties(
                VertexPropertyQuery {
                    inner: query.into(),
                    name: VERTEX_PROPERTY_HOLDER.into(),
                },
                &value,
            )
            .map_err(Error::UpdateVertex)
    }

    // deletes inbound and outbound edges as well
    fn delete_node(&self, node_id: NodeId) -> Result<()> {
        let trans = self.transaction()?;
        let query = SpecificVertexQuery { ids: vec![node_id] };
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
            .map_err(Error::DeleteVertex)
    }

    fn create_edge(&self, msg: Edge) -> Result<Edge> {
        let trans = self.transaction()?;
        let edge_type = Type::new(msg.edge_type.clone()).map_err(Error::CreateType)?;
        let edge_key = EdgeKey {
            outbound_id: msg.from,
            inbound_id: msg.to,
            t: edge_type,
        };
        let edge_id = indradb::util::generate_uuid_v1();
        dbg!(edge_id.clone());
        if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
            return Err(Error::CreateEdgeFailed);
        }
        let get_created_edge = SpecificEdgeQuery::single(edge_key);
        let query = EdgePropertyQuery {
            inner: get_created_edge.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_edge_properties(query, &msg.properties)
            .map_err(Error::SetEdgeProperties)?;
        // dbg! {&trans};

        Ok(Edge {
            edge_type: msg.edge_type,
            id: edge_id,
            from: msg.from,
            to: msg.to,
            properties: msg.properties,
        })
    }

    fn read_edge(&self, msg: Edge) -> Result<Edge> {
        let trans = self.transaction()?;
        let edge_key = msg.try_into()?;
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
        assert_eq!(properties.len(), 1);
        let properties = properties.pop().unwrap().value;

        Ok(Edge {
            edge_type: todo!(),
            id: todo!(),
            from: edge_key.outbound_id,
            to: edge_key.inbound_id,
            properties,
        })
    }

    fn update_edge(&self, (edge_id, value): (Edge, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;
        let edge_key = edge_id.try_into()?;
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: VERTEX_PROPERTY_HOLDER.into(),
        };
        trans
            .set_edge_properties(query, &value)
            .map_err(Error::UpdateEdgeProperties)?;

        Ok(())
    }

    fn delete_edge(&self, msg: Edge) -> Result<()> {
        let trans = self.transaction()?;
        let edge_key = msg.try_into()?;
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        trans.delete_edges(query).map_err(Error::DeleteEdge)?;
        Ok(())
    }

    fn transaction(&self) -> Result<MemoryTransaction> {
        self.datastore
            .transaction()
            .map_err(Error::CreateTransaction)
    }

    fn clear_database(&self) -> Result<()> {
        todo!()
    }
}

pub struct Config {
    pub db_path: String,
}

fn create_db(path: &str) -> std::result::Result<MemoryDatastore, bincode::Error> {
    if let Ok(db) = MemoryDatastore::read(path) {
        return Ok(db);
    }
    MemoryDatastore::create(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let cfg = Config {
            db_path: "newdb".into(),
        };
        let store = Store::new(&cfg).unwrap();

        let graph_id = store
            .execute(Msg::CreateGraph(serde_json::json!({
                "name": "first_graph",
            })))
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
            let reply = store.execute(make_msg_mut(MutateStateKind::CreateNode(
                Node::from_properties(properties),
            )));
            // dbg!(reply.clone());
            match reply {
                Reply::Node(node) => node,
                e => panic!("failed to create vertex: {:?}", e),
            }
        };

        let id1 = create_node(serde_json::json!({
            "name": "first_vertex",
        }));

        dbg!(id1.clone());

        print_state();

        print_state();

        store
            .execute(make_msg_mut(MutateStateKind::UpdateNode((
                id1.node_id,
                serde_json::json!({
                    "name": "updated_first_vertex",
                }),
            ))))
            .as_error()
            .unwrap();

        print_state();

        // let id2 = create_vertex(serde_json::json!({
        //     "name": "second_vertex",
        // }));

        // store
        //     .execute(make_msg_mut(MutateStateKind::CreateEdge(CreateEdge {
        //         directed: false,
        //         from: id1.clone(),
        //         edge_type: "edge_type1".into(),
        //         to: id2.clone(),
        //         properties: serde_json::json!({
        //             "name": "first_edge",
        //         }),
        //     })))
        //     .as_error()
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
