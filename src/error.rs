use crate::VertexId;
use std::result::Result as StdResult;

#[derive(Debug)]
pub enum Error {
    DatastoreCreateError(bincode::Error),
    CreateTransaction(indradb::Error),
    CreateVertex(indradb::Error),
    SetVertexProperties(indradb::Error),
    GetVertices(indradb::Error),
    GetEdgesOfVertex(indradb::Error),
    TypeNameTooLong,
    InvalidId(uuid::Error),
    CreateEdge(Option<indradb::Error>),
    UpdateVertex(indradb::Error),
}

impl From<uuid::Error> for Error {
    fn from(error: uuid::Error) -> Error {
        Error::InvalidId(error)
    }
}

pub type Result<T> = StdResult<T, Error>;
