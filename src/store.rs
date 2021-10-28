use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, MemoryDatastore, MemoryTransaction, RangeVertexQuery,
    SpecificEdgeQuery, SpecificVertexQuery, Transaction, Type, VertexPropertyQuery, VertexQuery,
    VertexQueryExt,
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

use crate::msg::{
    CreateEdge, CreateVertex, EdgeId, EdgeInfo, Graph, GraphId, Msg, MsgWithGraphId, Query, Reply,
    StateId, VertexId, VertexInfo,
};

use crate::error::{Error, Result};

const PROP_NAME: &str = "data";
const GRAPH_ROOT_TYPE: &str = "_root_type";
const GRAPH_ROOT_EDGE_TYPE: &str = "_root_edge_type";
const STATE_ID_PROPERTY: &str = "_state_id_prop";

#[derive(Debug)]
pub struct Store {
    datastore: MemoryDatastore,
    root_vertex_type: Type,
    root_edge_type: Type,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore = create_db(&cfg.db_path).map_err(Error::DatastoreCreateError)?;
        let store = Store {
            datastore: datastore,
            root_vertex_type: Type::new(GRAPH_ROOT_TYPE).unwrap(),
            root_edge_type: Type::new(GRAPH_ROOT_EDGE_TYPE).unwrap(),
        };
        Ok(store)
    }

    pub fn execute(&self, msg: MsgWithGraphId) -> Reply {
        if let Some(graph_id) = msg.graph_id.clone() {
            if let Err(e) = self.update_state_id(graph_id) {
                return Reply::Error(e.to_string());
            }
        }

        match msg.msg {
            Msg::CreateGraph(properties) => Reply::from_id(self.create_graph(properties)),
            Msg::ListGraphs => Reply::from_vertex_info_list(self.list_graphs()),
            Msg::Query(Query::ReadGraph(read_graph)) => {
                Reply::from_graph(self.read_graph(read_graph))
            }
            //
            Msg::CreateVertex(create_vertex) => {
                Reply::from_id(self.create_vertex((msg.graph_id, create_vertex)))
            }
            Msg::ReadVertex(msg) => Reply::from_vertex_info(self.read_vertex(&msg)),
            Msg::UpdateVertex(msg) => Reply::from_empty(self.update_vertex(msg)),
            Msg::DeleteVertex(msg) => Reply::from_empty(self.delete_vertex(msg)),
            Msg::CreateEdge(msg) => Reply::from_empty(self.create_edge(msg)),
            Msg::ReadEdge(msg) => Reply::from_edge_info(self.read_edge(msg)),
            Msg::UpdateEdge(msg) => Reply::from_empty(self.update_edge(msg)),
            Msg::DeleteEdge(msg) => Reply::from_empty(self.delete_edge(msg)),
            _ => todo!(),
        }
    }

    fn update_state_id(&self, graph_id: GraphId) -> Result<()> {
        let state_id = JsonValue::String(Uuid::new_v4().to_string());
        let mut graph_root = self.read_vertex(&graph_id)?;
        graph_root
            .properties
            .as_object_mut()
            .unwrap()
            .insert(STATE_ID_PROPERTY.into(), state_id);
        self.update_vertex((graph_id, graph_root.properties))?;
        Ok(())
    }

    fn create_graph(&self, properties: JsonValue) -> Result<VertexId> {
        let mut properties = properties;
        let state_id = JsonValue::String(Uuid::new_v4().to_string());
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

    fn read_graph(
        &self,
        (graph_id, expected_state_id): (GraphId, StateId),
    ) -> Result<Option<Graph>> {
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
            .as_str()
            .unwrap()
            .into();
        let graph = if state_id == expected_state_id {
            None
        } else {
            Some(Graph { vertices, state_id })
        };
        Ok(graph)
    }

    fn create_vertex(&self, (graph_id, msg): (Option<GraphId>, CreateVertex)) -> Result<VertexId> {
        let trans = self.transaction()?;
        let vertex_type = Type::new(msg.vertex_type).map_err(Error::CreateType)?;
        let uuid = trans
            .create_vertex_from_type(vertex_type)
            .map_err(Error::CreateVertex)?;
        let query = SpecificVertexQuery {
            ids: vec![uuid.clone()],
        }
        .into();
        let query = VertexPropertyQuery {
            inner: query,
            name: PROP_NAME.into(),
        };
        trans
            .set_vertex_properties(query, &msg.properties)
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

    fn read_vertex(&self, vertex_id: &VertexId) -> Result<VertexInfo> {
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

    fn update_vertex(&self, (vertex_id, value): (VertexId, JsonValue)) -> Result<()> {
        let trans = self.transaction()?;
        let uuid = Uuid::parse_str(vertex_id.as_str())?;
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
        dbg! {&trans};

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

#[test]
fn test() {
    let cfg = Config {
        db_path: "newdb".into(),
    };
    let store = Store::new(&cfg).unwrap();

    let graph_id = store
        .execute(MsgWithGraphId {
            graph_id: None,
            msg: Msg::CreateGraph(serde_json::json!({
                "name": "first_graph",
            })),
        })
        .as_id()
        .unwrap()
        .to_string();

    dbg!(graph_id.clone());

    let make_msg = |msg: Msg| MsgWithGraphId {
        msg,
        graph_id: Some(graph_id.clone()),
    };

    let print_state = || {
        let reply = store.execute(make_msg(Msg::Query(Query::ReadGraph((
            graph_id.clone(),
            "".into(),
        )))));
        dbg!(reply);
    };

    let create_vertex = |properties: serde_json::Value| {
        let reply = store.execute(make_msg(Msg::CreateVertex(CreateVertex {
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

    let reply = store.execute(make_msg(Msg::UpdateVertex((
        id1.clone(),
        serde_json::json!({
            "name": "updated_first_vertex",
        }),
    ))));

    print_state();

    /*
    //




    println!("{:#?}", reply);

    let id2 = create_vertex(serde_json::json!({
        "name": "second_vertex",
    }));
    store.execute(Msg::CreateEdge(CreateEdge {
        directed: false,
        from: store.graph.root_node_id.clone(),
        edge_type: "edge_type1".into(),
        to: id2.clone(),
        properties: serde_json::json!({
            "name": "first_edge",
        }),
    }));

    println!("{}", id2);

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
