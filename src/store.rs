use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, MemoryDatastore, MemoryTransaction, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, VertexPropertyQuery, VertexQuery, VertexQueryExt,
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use uuid::Uuid;

use crate::msg::{
    CreateEdge, CreateVertex, EdgeId, EdgeInfo, GraphId, GraphResult, Msg, Query, Reply, VertexId,
    VertexInfo,
};

use crate::error::{Error, Result};

const PROP_NAME: &str = "data";
const GRAPH_ROOT_TYPE: &str = "_root_type";

#[derive(Debug)]
pub struct Store {
    datastore: MemoryDatastore,
    graph: Graph,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub root_node_id: VertexId,
}

impl Graph {
    fn new(store: &Store) -> Self {
        let root_properties = serde_json::json!({
            "name": "root_node",
        });

        let contruct_graph = |properties: serde_json::Value| {
            let reply = store.execute(Msg::CreateVertex((
                "1".into(),
                CreateVertex {
                    vertex_type: GRAPH_ROOT_TYPE.into(),
                    properties,
                },
            )));
            match reply {
                Reply::Id(id) => id,
                e => panic!("failed to create vertex: {:?}", e),
            }
        };

        let graph_id = contruct_graph(root_properties);

        Self {
            root_node_id: graph_id,
        }
    }
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        let datastore = create_db(&cfg.db_path).map_err(Error::DatastoreCreateError)?;
        let mut store = Store {
            datastore: datastore,
            graph: Graph::default(),
        };
        store.graph = Graph::new(&store);
        Ok(store)
    }

    // pub fn create_graph(&mut self) -> Result<VertexId> {
    //     let graph = Graph::new(&self);
    //     Ok(graph.root_node_id)
    // }

    pub fn execute(&self, msg: Msg) -> Reply {
        match msg {
            // Msg::ListGraph =>
            // Msg::CreateGraph => Reply::from_id(self.create_graph()),
            Msg::CreateVertex(msg) => Reply::from_id(self.create_vertex(msg)),
            Msg::ReadVertex(msg) => Reply::from_vertex_info(self.read_vertex(&msg)),
            Msg::UpdateVertex(msg) => Reply::from_empty(self.update_vertex(msg)),
            Msg::DeleteVertex(msg) => Reply::from_empty(self.delete_vertex(msg)),
            Msg::CreateEdge(msg) => Reply::from_empty(self.create_edge(msg)),
            Msg::ReadEdge(msg) => Reply::from_edge_info(self.read_edge(msg)),
            Msg::UpdateEdge(msg) => Reply::from_empty(self.update_edge(msg)),
            Msg::DeleteEdge(msg) => Reply::from_empty(self.delete_edge(msg)),
            Msg::Query(Query::GetAll(msg)) => Reply::from_graph(self.get_all_nodes_and_edges()),
            _ => todo!(),
        }
    }

    fn get_all_nodes_and_edges(&self) -> Result<GraphResult> {
        let vertices = self
            .read_vertex(&self.graph.root_node_id)?
            .outbound_edges
            .iter()
            .map(|edge_id| self.read_vertex(&edge_id.to))
            .collect::<Result<Vec<_>>>()?;

        Ok(GraphResult { vertices })
    }

    fn create_vertex(&self, (_, msg): (GraphId, CreateVertex)) -> Result<String> {
        let trans = self.transaction()?;
        let vertex_type = Type::new(msg.vertex_type).map_err(|_| Error::TypeNameTooLong)?;
        let uuid = trans
            .create_vertex_from_type(vertex_type)
            .map_err(Error::CreateVertex)?;
        let query = SpecificVertexQuery { ids: vec![uuid] }.into();
        let query = VertexPropertyQuery {
            inner: query,
            name: PROP_NAME.into(),
        };
        trans
            .set_vertex_properties(query, &msg.properties)
            .map_err(Error::SetVertexProperties)?;

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
        let outbound_edges = trans
            .get_edges(outbound_query)
            .map_err(Error::GetEdgesOfVertex)?
            .into_iter()
            .map(|edge| EdgeId::from(edge.key))
            .collect();
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
        let edge_type = Type::new(msg.edge_type).map_err(|_| Error::TypeNameTooLong)?;
        let edge_key = EdgeKey {
            outbound_id: Uuid::parse_str(msg.from.as_str())?,
            inbound_id: Uuid::parse_str(msg.to.as_str())?,
            t: edge_type,
        };
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
    //
    let create_vertex = |properties: serde_json::Value| {
        let reply = store.execute(Msg::CreateVertex((
            store.graph.root_node_id.clone(),
            CreateVertex {
                vertex_type: GRAPH_ROOT_TYPE.into(),
                properties,
            },
        )));
        match reply {
            Reply::Id(id) => id,
            e => panic!("failed to create vertex: {:?}", e),
        }
    };

    let id1 = create_vertex(serde_json::json!({
        "name": "first_vertex",
    }));

    let connect_to_root = store.execute(Msg::CreateEdge(CreateEdge {
        directed: false,
        from: store.graph.root_node_id.clone(),
        edge_type: "edge_type1".into(),
        to: id1.clone(),
        properties: serde_json::json!({
            "name": "first_edge",
        }),
    }));
    dbg!(connect_to_root);

    println!("{}", id1);

    let reply = store.execute(Msg::UpdateVertex((
        id1.clone(),
        serde_json::json!({
            "name": "updated_first_vertex",
        }),
    )));

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
    dbg! {read};

    let get_all = store.get_all_nodes_and_edges();
    dbg! {get_all};
}
