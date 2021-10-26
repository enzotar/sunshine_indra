use indradb::{
    Datastore, EdgeKey, EdgePropertyQuery, MemoryDatastore, MemoryTransaction, SpecificEdgeQuery,
    SpecificVertexQuery, Transaction, Type, VertexPropertyQuery, VertexQuery, VertexQueryExt,
};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
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
            Msg::DeleteVertex(msg) => Reply::from_empty(self.delete_vertex(msg)),
            Msg::CreateEdge(msg) => Reply::from_empty(self.create_edge(msg)),
            Msg::ReadEdge(msg) => Reply::from_edge_info(self.read_edge(msg)),
            Msg::UpdateEdge(msg) => Reply::from_empty(self.update_edge(msg)),
            Msg::DeleteEdge(msg) => Reply::from_empty(self.delete_edge(msg)),
            Msg::ReverseEdge(_msg) => todo!(),
            Msg::ClearDatabase => Reply::from_empty(self.clear_database()),
            // Msg::Init => Reply::from_db_state(self.init()),
        }
    }

    // fn init(&self) -> Result<DbState> {
    //     let trans = self.transaction()?;
    //     let query = RangeVertexQuery {
    //         limit: 0,
    //         t: None,
    //         start_id: None,
    //     };
    //     let vertices = trans
    //         .get_vertices(query)
    //         .map_err(Error::GetVertices)?
    //         .into_iter()
    //         .map(|vertex| self.read_vertex(vertex.id.into()))
    //         .collect();

    //     let edges = raw_vertices
    //         .into_iter()
    //         .map(|vert| {
    //             let query = SpecificVertexQuery { ids: vec![vert.id] };
    //             trans
    //                 .get_edges(query.outbound())
    //                 .map_err(Error::GetEdgesOfVertex)?
    //                 .into_iter()
    //         })
    //         .chain()
    //         .map(|edge| self.read_edge(EdgeId::from(edge.key)))
    //         .collect();

    //     Ok(DbState { edges, vertices })
    // }

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

pub enum Msg {
    // Init,
    CreateVertex(CreateVertex),
    ReadVertex(VertexId),
    UpdateVertex((VertexId, JsonValue)),
    DeleteVertex(VertexId),
    CreateEdge(CreateEdge),
    ReadEdge(EdgeId),
    UpdateEdge((EdgeId, JsonValue)),
    DeleteEdge(EdgeId),
    ReverseEdge(EdgeId),
    ClearDatabase,
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

impl From<EdgeKey> for EdgeId {
    fn from(edge_key: EdgeKey) -> EdgeId {
        EdgeId {
            from: edge_key.outbound_id.to_string(),
            to: edge_key.inbound_id.to_string(),
            edge_type: edge_key.t.0,
        }
    }
}

impl TryInto<EdgeKey> for EdgeId {
    type Error = Error;

    fn try_into(self) -> Result<EdgeKey> {
        Ok(EdgeKey {
            outbound_id: Uuid::parse_str(&self.from)?,
            inbound_id: Uuid::parse_str(&self.to)?,
            t: Type(self.edge_type),
        })
    }
}

type EdgeInfo = JsonValue;

#[derive(Debug)]
pub enum Reply {
    // DbState(DbState),
    Id(String),
    Error(String),
    VertexInfo(VertexInfo),
    EdgeInfo(EdgeInfo),
    Empty,
}

struct DbState {
    edges: Vec<EdgeInfo>,
    vertices: Vec<VertexInfo>,
}

impl Reply {
    fn from_id(id: Result<String>) -> Reply {
        match id {
            Ok(id) => Reply::Id(id),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    fn from_empty(val: Result<()>) -> Reply {
        match val {
            Ok(_) => Reply::Empty,
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    fn from_vertex_info(info: Result<VertexInfo>) -> Reply {
        match info {
            Ok(info) => Reply::VertexInfo(info),
            Err(e) => Reply::Error(e.to_string()),
        }
    }

    fn from_edge_info(info: Result<EdgeInfo>) -> Reply {
        match info {
            Ok(info) => Reply::EdgeInfo(info),
            Err(e) => Reply::Error(e.to_string()),
        }
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
    fn test_create_vertex() {}
}