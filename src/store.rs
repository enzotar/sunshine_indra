use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, RangeVertexQuery, SledDatastore, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, VertexPropertyQuery, VertexQuery, VertexQueryExt,
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

use crate::msg::{
    CreateEdge, CreateVertex, EdgeId, EdgeInfo, Graph, GraphId, Msg, MutateState, MutateStateKind,
    ReadOnly, Reply, VertexId, VertexInfo,
};

use crate::error::{Error, Result};

const PROP_NAME: &str = "data";
const GRAPH_ROOT_TYPE: &str = "_root_type";
const GRAPH_ROOT_EDGE_TYPE: &str = "_root_edge_type";
const STATE_ID_PROPERTY: &str = "_state_id_prop";

pub struct Store {
    datastore: SledDatastore,
    root_vertex_type: Type,
    root_edge_type: Type,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore = SledDatastore::new(&cfg.db_path).map_err(Error::DatastoreCreate)?;
        let store = Store {
            datastore,
            root_vertex_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
            root_edge_type: Type::new(GRAPH_ROOT_EDGE_TYPE).unwrap(),
        };
        Ok(store)
        //         return Reply::Error(e.to_string());
    }

    pub fn execute(&self, msg: Msg) -> Reply {
        match msg {
            Msg::MutateState(mutate_state) => self.execute_mutate_state(mutate_state),
            Msg::ReadOnly(read_only) => self.execute_read_only(read_only),
            Msg::CreateGraph(properties) => Reply::from_id(self.create_graph(properties)),
        }
    }

    fn execute_mutate_state(&self, msg: MutateState) -> Reply {
        let MutateState { kind, graph_id } = msg;

        let reply = match kind {
            MutateStateKind::CreateVertex(create_vertex) => {
                Reply::from_id(self.create_vertex((Some(&graph_id), create_vertex)))
            }
            MutateStateKind::UpdateVertex((vertex_id, properties)) => {
                Reply::from_empty(self.update_vertex((&vertex_id, properties)))
            }
            MutateStateKind::DeleteVertex(msg) => Reply::from_empty(self.delete_vertex(msg)),
            MutateStateKind::CreateEdge(msg) => Reply::from_empty(self.create_edge(msg)),
            MutateStateKind::UpdateEdge(msg) => Reply::from_empty(self.update_edge(msg)),
            MutateStateKind::DeleteEdge(msg) => Reply::from_empty(self.delete_edge(msg)),
            MutateStateKind::DeleteGraph(_) => todo!(),
            MutateStateKind::ReverseEdge(_) => todo!(),
        };

        if let Reply::Error(e) = reply {
            return Reply::Error(e);
        }

        if let Err(e) = self.update_state_id(&graph_id) {
            return Reply::Error(e.to_string());
        }

        reply
    }

    fn execute_read_only(&self, msg: ReadOnly) -> Reply {
        match msg {
            ReadOnly::ReadEdge(msg) => Reply::from_edge_info(self.read_edge(msg)),
            ReadOnly::ReadVertex(msg) => Reply::from_vertex_info(self.read_vertex(&msg)),
            ReadOnly::ReadGraph(read_graph) => Reply::from_graph(self.read_graph(read_graph)),
            ReadOnly::ListGraphs => Reply::from_vertex_info_list(self.list_graphs()),
        }
    }

    fn update_state_id(&self, graph_id: &str) -> Result<()> {
        let mut graph_root = self.read_vertex(graph_id)?;
        let properties = graph_root.properties.as_object_mut().unwrap();
        let old_id = properties.get(STATE_ID_PROPERTY).unwrap().as_u64().unwrap();
        let new_id = JsonValue::Number(serde_json::Number::from(old_id + 1));

        properties.insert(STATE_ID_PROPERTY.into(), new_id);

        self.update_vertex((graph_id, graph_root.properties))?;

        Ok(())
    }

    fn create_graph(&self, properties: JsonValue) -> Result<VertexId> {
        let mut properties = properties;
        let state_id = JsonValue::Number(serde_json::Number::from(0u64));
        properties
            .as_object_mut()
            .unwrap()
            .insert(STATE_ID_PROPERTY.into(), state_id);
        self.create_vertex((
            None,
            CreateVertex {
                vertex_type: GRAPH_ROOT_TYPE.into(),
                properties,
            },
        ))
    }

    fn list_graphs(&self) -> Result<Vec<VertexInfo>> {
        let trans = self.transaction()?;
        trans
            .get_vertices(RangeVertexQuery {
                limit: 0,
                t: Some(self.root_vertex_type.clone()),
                start_id: None,
            })
            .map_err(Error::GetVertices)?
            .iter()
            .map(|vertex| self.read_vertex(&vertex.id.to_string()))
            .collect::<Result<Vec<_>>>()
    }

    fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let graph_info = self.read_vertex(&graph_id)?;
        let vertices = graph_info
            .outbound_edges
            .iter()
            .map(|edge_id| self.read_vertex(&edge_id.to))
            .collect::<Result<Vec<_>>>()?;
        let state_id = graph_info
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

    fn create_vertex(
        &self,
        (graph_id, create_vertex): (Option<&GraphId>, CreateVertex),
    ) -> Result<VertexId> {
        let trans = self.transaction()?;
        let vertex_type = Type::new(create_vertex.vertex_type).map_err(Error::CreateType)?;
        let uuid = trans
            .create_vertex_from_type(vertex_type)
            .map_err(Error::CreateVertex)?;
        let query = SpecificVertexQuery { ids: vec![uuid] }.into();
        let query = VertexPropertyQuery {
            inner: query,
            name: PROP_NAME.into(),
        };
        trans
            .set_vertex_properties(query, &create_vertex.properties)
            .map_err(Error::SetVertexProperties)?;

        if let Some(graph_id) = graph_id {
            let edge_key = EdgeKey {
                outbound_id: Uuid::parse_str(graph_id.as_str())?,
                inbound_id: uuid,
                t: self.root_edge_type.clone(),
            };
            if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
                return Err(Error::CreateEdgeFailed);
            }
        }

        Ok(uuid.to_string())
    }

    fn read_vertex(&self, vertex_id: &str) -> Result<VertexInfo> {
        let trans = self.transaction()?;
        let uuid = Uuid::parse_str(vertex_id)?;
        let query = SpecificVertexQuery { ids: vec![uuid] };
        let outbound_query = query.clone().outbound();
        let inbound_query = query.clone().inbound();
        let mut properties = trans
            .get_all_vertex_properties(VertexQuery::Specific(query))
            .map_err(Error::GetVertices)?;
        assert_eq!(properties.len(), 1);
        let properties = properties.pop().unwrap().props.pop().unwrap().value;
        //dbg! {&properties};
        let outbound_edges = trans
            .get_edges(outbound_query)
            .map_err(Error::GetEdgesOfVertex)?
            .into_iter()
            .map(|edge| EdgeId::from(edge.key))
            .collect();
        //dbg! {&outbound_edges}; //////////////////////////////////////////////////////////////
        let inbound_edges = trans
            .get_edges(inbound_query)
            .map_err(Error::GetEdgesOfVertex)?
            .into_iter()
            .map(|edge| EdgeId::from(edge.key))
            .collect();
        Ok(VertexInfo {
            outbound_edges,
            inbound_edges,
            properties,
        })
    }

    fn update_vertex(&self, (vertex_id, value): (&str, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;
        let uuid = Uuid::parse_str(vertex_id)?;
        let query = SpecificVertexQuery { ids: vec![uuid] };
        trans
            .set_vertex_properties(
                VertexPropertyQuery {
                    inner: query.into(),
                    name: PROP_NAME.into(),
                },
                &value,
            )
            .map_err(Error::UpdateVertex)
    }

    // deletes inbound and outbound edges as well
    fn delete_vertex(&self, vertex_id: VertexId) -> Result<()> {
        let trans = self.transaction()?;
        let uuid = Uuid::parse_str(vertex_id.as_str())?;
        let query = SpecificVertexQuery { ids: vec![uuid] };
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

    fn create_edge(&self, msg: CreateEdge) -> Result<()> {
        let trans = self.transaction()?;
        let edge_type = Type::new(msg.edge_type).map_err(Error::CreateType)?;
        let edge_key = EdgeKey {
            outbound_id: Uuid::parse_str(msg.from.as_str())?,
            inbound_id: Uuid::parse_str(msg.to.as_str())?,
            t: edge_type,
        };
        if !trans.create_edge(&edge_key).map_err(Error::CreateEdge)? {
            return Err(Error::CreateEdgeFailed);
        }
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: PROP_NAME.into(),
        };
        trans
            .set_edge_properties(query, &msg.properties)
            .map_err(Error::SetEdgeProperties)?;

        Ok(())
    }

    fn read_edge(&self, msg: EdgeId) -> Result<EdgeInfo> {
        let trans = self.transaction()?;
        let edge_key = msg.try_into()?;
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: PROP_NAME.into(),
        };
        let mut properties = trans
            .get_edge_properties(query)
            .map_err(Error::GetEdgeProperties)?;
        assert_eq!(properties.len(), 1);
        let properties = properties.pop().unwrap().value;
        Ok(properties)
    }

    fn update_edge(&self, (edge_id, value): (EdgeId, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;
        let edge_key = edge_id.try_into()?;
        let query = SpecificEdgeQuery {
            keys: vec![edge_key],
        };
        let query = EdgePropertyQuery {
            inner: query.into(),
            name: PROP_NAME.into(),
        };
        trans
            .set_edge_properties(query, &value)
            .map_err(Error::UpdateEdgeProperties)?;

        Ok(())
    }

    fn delete_edge(&self, msg: EdgeId) -> Result<()> {
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
            .as_id()
            .unwrap()
            .to_string();

        dbg!(graph_id.clone());

        let make_msg_mut = |kind: MutateStateKind| {
            Msg::MutateState(MutateState {
                kind,
                graph_id: graph_id.clone(),
            })
        };

        let print_state = || {
            let reply = store.execute(Msg::ReadOnly(ReadOnly::ReadGraph(graph_id.clone())));
            dbg!(reply);
        };

        let create_vertex = |properties: serde_json::Value| {
            let reply = store.execute(make_msg_mut(MutateStateKind::CreateVertex(CreateVertex {
                vertex_type: GRAPH_ROOT_TYPE.into(),
                properties,
            })));
            match reply {
                Reply::Id(id) => id,
                e => panic!("failed to create vertex: {:?}", e),
            }
        };

        let id1 = create_vertex(serde_json::json!({
            "name": "first_vertex",
        }));

        dbg!(id1.clone());

        print_state();

        print_state();

        store
            .execute(make_msg_mut(MutateStateKind::UpdateVertex((
                id1.clone(),
                serde_json::json!({
                    "name": "updated_first_vertex",
                }),
            ))))
            .as_error()
            .unwrap();

        print_state();

        let id2 = create_vertex(serde_json::json!({
            "name": "second_vertex",
        }));

        store
            .execute(make_msg_mut(MutateStateKind::CreateEdge(CreateEdge {
                directed: false,
                from: id1.clone(),
                edge_type: "edge_type1".into(),
                to: id2.clone(),
                properties: serde_json::json!({
                    "name": "first_edge",
                }),
            })))
            .as_error()
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
