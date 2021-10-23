use std::result::Result as StdResult;

#[derive(Debug)]
pub enum Error {
    DatastoreCreateError(bincode::Error),
    CreateTransaction(indradb::Error),
    CreateVertex(indradb::Error),
    SetVertexProperties(indradb::Error),
}

pub type Result<T> = StdResult<T, Error>;
