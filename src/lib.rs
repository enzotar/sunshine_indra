use indradb::{
    Datastore, EdgeKey, MemoryDatastore, MemoryTransaction, RangeVertexQuery, SpecificVertexQuery,
    Transaction, Type, Vertex, VertexPropertyQuery, VertexQuery, VertexQueryExt,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

mod error;

pub use error::{Error, Result};

const PROP_NAME: &str = "data";

#[derive(Debug)]
pub struct Store {
    datastore: MemoryDatastore,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        Ok(Store {
            datastore: create_db(&cfg.db_path).map_err(Error::DatastoreCreateError)?,
        })
    }

    pub fn execute(&self, msg: Msg) -> Reply {
        match msg {
            Msg::CreateVertex(msg) => Reply::from_id(self.create_vertex(msg)),
            Msg::ReadVertex(msg) => Reply::from_vertex_info(self.read_vertex(msg)),
            Msg::UpdateVertex(msg) => Reply::from_empty(self.update_vertex(msg)),
            Msg::CreateEdge(msg) => Reply::from_empty(self.create_edge(msg)),
            _ => todo!(),
        }
    }

    fn create_vertex(&self, msg: CreateVertex) -> Result<String> {
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

    fn read_vertex(&self, vertex_id: VertexId) -> Result<VertexInfo> {
        let trans = self.transaction()?;
        let uuid = Uuid::parse_str(vertex_id.as_str())?;
        let query = SpecificVertexQuery { ids: vec![uuid] };
        let outbound_query = query.clone().outbound();
        let inbound_query = query.clone().inbound();
        let mut properties = trans
            .get_all_vertex_properties(VertexQuery::Specific(query))
            .map_err(Error::GetVertices)?;
        assert_eq!(properties.len(), 1);
        let properties = properties.pop().unwrap().props.pop().unwrap().value;
        let convert_edge = |edge: indradb::Edge| EdgeId {
            from: edge.key.outbound_id.to_string(),
            to: edge.key.inbound_id.to_string(),
            edge_type: edge.key.t.0,
        };
        let outbound_edges = trans
            .get_edges(outbound_query)
            .map_err(Error::GetEdgesOfVertex)?
            .into_iter()
            .map(convert_edge)
            .collect();
        let inbound_edges = trans
            .get_edges(inbound_query)
            .map_err(Error::GetEdgesOfVertex)?
            .into_iter()
            .map(convert_edge)
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

    fn create_edge(&self, msg: CreateEdge) -> Result<()> {
        let trans = self.transaction()?;
        let edge_type = Type::new(msg.edge_type).map_err(|_| Error::TypeNameTooLong)?;
        let edge_key = EdgeKey {
            outbound_id: Uuid::parse_str(msg.from.as_str())?,
            inbound_id: Uuid::parse_str(msg.to.as_str())?,
            t: edge_type,
        };
        if !trans
            .create_edge(&edge_key)
            .map_err(|e| Error::CreateEdge(Some(e)))?
        {
            return Err(Error::CreateEdge(None));
        }
        Ok(())
    }

    fn transaction(&self) -> Result<MemoryTransaction> {
        self.datastore
            .transaction()
            .map_err(Error::CreateTransaction)
    }
}

pub enum Msg {
    CreateVertex(CreateVertex),
    ReadVertex(VertexId),
    UpdateVertex((VertexId, JsonValue)),
    DeleteVertex(VertexId),
    CreateEdge(CreateEdge),
    ReadEdge(EdgeId),
    UpdateEdge((EdgeId, JsonValue)),
    DeleteEdge(EdgeId),
    ReverseEdge(EdgeId),
    GetEdgesOfVertex(VertexId),
}
pub struct CreateVertex {
    pub vertex_type: String,
    pub properties: JsonValue,
}

#[derive(Debug)]
pub struct VertexInfo {
    pub outbound_edges: Vec<EdgeId>,
    pub inbound_edges: Vec<EdgeId>,
    pub properties: JsonValue,
}

pub struct CreateEdge {
    pub directed: bool,
    pub from: VertexId,
    pub edge_type: String,
    pub to: VertexId,
    pub properties: JsonValue,
}

type VertexId = String;

#[derive(Debug)]
pub struct EdgeId {
    pub from: VertexId,
    pub to: VertexId,
    pub edge_type: String,
}

#[derive(Debug)]
pub enum Reply {
    Id(String),
    Error(String),
    VertexInfo(VertexInfo),
    Empty,
}

impl Reply {
    fn from_id(id: Result<String>) -> Reply {
        match id {
            Ok(id) => Reply::Id(id),
            Err(e) => Reply::from(e),
        }
    }

    fn from_empty(val: Result<()>) -> Reply {
        match val {
            Ok(_) => Reply::Empty,
            Err(e) => Reply::from(e),
        }
    }

    fn from_vertex_info(info: Result<VertexInfo>) -> Reply {
        match info {
            Ok(info) => Reply::VertexInfo(info),
            Err(e) => Reply::from(e),
        }
    }
}

impl From<Error> for Reply {
    fn from(err: Error) -> Reply {
        Reply::Error(format!("{:#?}", err))
    }
}

pub struct Config {
    pub db_path: String,
}

fn create_db(path: &str) -> std::result::Result<MemoryDatastore, bincode::Error> {
    match MemoryDatastore::read(path) {
        Ok(db) => return Ok(db),
        Err(_) => (),
    }
    MemoryDatastore::create(path)
}
