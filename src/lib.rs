use indradb::{
    Datastore, MemoryDatastore, RangeVertexQuery, SpecificVertexQuery, Transaction,
    Type as VertexType, Vertex, VertexPropertyQuery, VertexQuery,
};
use serde_json::Value as JsonValue;

mod error;

pub use error::{Error, Result};

#[derive(Debug)]
pub struct Store {
    datastore: MemoryDatastore,
    vertex_type: VertexType,
}

impl Store {
    pub fn new(cfg: &Config) -> Result<Store> {
        Ok(Store {
            datastore: create_db(&cfg.db_path).map_err(Error::DatastoreCreateError)?,
            vertex_type: VertexType::new("_indra_vertex_type").unwrap(),
        })
    }

    pub fn execute(&self, msg: Msg) -> Result<String> {
        match msg {
            Msg::CreateVertex(msg) => self.create_vertex(msg),
            Msg::CreateEdge(msg) => self.create_edge(msg),
        }
    }

    fn create_vertex(&self, msg: CreateVertex) -> Result<String> {
        let trans = self
            .datastore
            .transaction()
            .map_err(Error::CreateTransaction)?;
        let uuid = trans
            .create_vertex_from_type(self.vertex_type.clone())
            .map_err(Error::CreateVertex)?;
        let query = VertexQuery::Specific(SpecificVertexQuery { ids: vec![uuid] });
        let query = VertexPropertyQuery {
            inner: query,
            name: "data".into(),
        };
        trans
            .set_vertex_properties(query, &msg.data)
            .map_err(Error::SetVertexProperties)?;

        Ok(uuid.to_string())
    }

    fn create_edge(&self, msg: CreateEdge) -> Result<String> {
        // let trans = self
        //     .datastore
        //     .transaction()
        //     .map_err(Error::CreateTransaction)?;
        todo!();
    }
}

pub enum Msg {
    CreateVertex(CreateVertex),
    CreateEdge(CreateEdge),
}

pub struct CreateVertex {
    pub data: JsonValue,
}

pub struct CreateEdge {}

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
