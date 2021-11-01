use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, RangeVertexQuery, SledDatastore, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, Vertex, VertexPropertyQuery, VertexQuery,
    VertexQueryExt,
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
const STATE_ID_PROPERTY: &str = "_state_id_prop";

pub struct Store {
    datastore: SledDatastore,
    root_node_type: Type,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore = SledDatastore::new(&cfg.db_path).map_err(Error::DatastoreCreate)?;
        let store = Store {
            datastore,
            root_node_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
        };
        Ok(store)
        //         return Reply::Error(e.to_string());
    }

    pub fn execute(&self, msg: Msg) -> Result<Reply> {
        match msg {
            Msg::CreateGraph(properties) => self.create_graph(properties).map(Reply::Node),
            Msg::MutateState(mutate_state) => self.execute_mutate_state(mutate_state),
            Msg::Query(read_only) => self.execute_read_only(read_only),
        }
    }

    fn execute_mutate_state(&self, msg: MutateState) -> Result<Reply> {
        let MutateState { kind, graph_id } = msg;

        let reply = match kind {
            MutateStateKind::CreateNode(node_args) => self
                .create_node((Some(graph_id), node_args))
                .map(Reply::Node),
            MutateStateKind::UpdateNode((node_id, properties)) => self
                .update_node((node_id, properties))
                .map(|_| Reply::Empty),
            MutateStateKind::DeleteNode(msg) => self.delete_node(msg.node_id).map(|_| Reply::Empty),
            MutateStateKind::CreateEdge(msg) => self.create_edge(msg).map(Reply::Edge),
            MutateStateKind::UpdateEdge(msg) => self.update_edge(msg).map(|_| Reply::Empty),
            MutateStateKind::DeleteEdge(msg) => self.delete_edge(msg).map(|_| Reply::Empty),
            MutateStateKind::DeleteGraph(_) => todo!(),
            MutateStateKind::ReverseEdge(_) => todo!(),
        };

        self.update_state_id(graph_id)?;

        reply
    }

    fn execute_read_only(&self, msg: Query) -> Result<Reply> {
        match msg {
            Query::ReadEdge(msg) => self.read_edge(msg).map(Reply::Edge),
            Query::ReadNode(msg) => self.read_node(msg).map(Reply::Node),
            Query::ReadGraph(read_graph) => self.read_graph(read_graph).map(Reply::Graph),
            Query::ListGraphs => self.list_graphs().map(Reply::NodeList),
            _ => todo!(),
        }
    }

    fn update_state_id(&self, graph_id: Uuid) -> Result<()> {
        let mut graph_root = self.read_node(graph_id)?;
        let properties = graph_root.properties.as_object_mut().unwrap();
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
            .map_err(Error::GetNodes)?
            .iter()
            .map(|node| self.read_node(node.id))
            .collect::<Result<Vec<_>>>()
    }

    fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let graph_node = self.read_node(graph_id)?;
        let vertices = graph_node
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

        Ok(Graph { vertices, state_id })
    }

    // does user have latest state?

    fn create_node(&self, (graph_id, node_args): (Option<GraphId>, Node)) -> Result<Node> {
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
            .set_vertex_properties(vertex_property_query, &node_args.properties)
            .map_err(Error::SetNodeProperties)?;

        if let Some(graph_id) = graph_id {
            let edge_key = EdgeKey {
                outbound_id: graph_id.clone(),
                inbound_id: node.id,
                t: Type(indradb::util::generate_uuid_v1().to_string()),
            };
            if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
                return Err(Error::CreateEdgeFailed);
            }
        }
        Ok(Node {
            node_id: node.id,
            properties: node_args.properties,
            outbound_edges: Vec::new(),
            inbound_edges: Vec::new(), // should have graph
        })
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
            .map_err(Error::UpdateNode)
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
            .map_err(Error::DeleteNode)
    }

    fn create_edge(&self, msg: Edge) -> Result<Edge> {
        let trans = self.transaction()?;
        let edge_id = indradb::util::generate_uuid_v1();
        let edge_type = Type::new(edge_id.to_string().clone()).map_err(Error::CreateType)?;
        let edge_key = EdgeKey {
            outbound_id: msg.from,
            inbound_id: msg.to,
            t: edge_type,
        };
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

        Ok(Edge {
            id: edge_id,
            from: msg.from,
            to: msg.to,
            properties: msg.properties,
        })
    }

    fn read_edge(&self, msg: Edge) -> Result<Edge> {
        let trans = self.transaction()?;
        let edge_key: EdgeKey = msg.try_into()?;
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

    fn update_edge(&self, (edge, value): (Edge, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;
        let edge_key = edge.try_into()?;
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
        let store = Store::new(&cfg).unwrap();

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
                .execute(make_msg_mut(MutateStateKind::CreateNode(
                    Node::from_properties(properties),
                )))
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

        store
            .execute(make_msg_mut(MutateStateKind::UpdateNode((
                id1,
                serde_json::json!({
                    "name": "updated_first_vertex",
                }),
            ))))
            .unwrap();

        print_state();

        let id2 = create_node(serde_json::json!({
            "name": "second_vertex",
        }));

        store
            .execute(make_msg_mut(MutateStateKind::CreateEdge(Edge {
                from: id1.clone(),
                to: id2.clone(),
                properties: serde_json::json!({
                    "name": "first_edge",
                }),
                id: indradb::util::generate_uuid_v1(),
            })))
            .unwrap();

        store
            .execute(make_msg_mut(MutateStateKind::CreateEdge(Edge {
                from: id1.clone(),
                to: id2.clone(),
                properties: serde_json::json!({
                    "name": "second_edge",
                }),
                id: indradb::util::generate_uuid_v1(),
            })))
            .unwrap();

        print_state();

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
