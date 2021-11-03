use std::result::Result as StdResult;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("error while creating datastore: {0}.")]
    DatastoreCreate(indradb::Error),
    #[error("error while creating transaction: {0}.")]
    CreateTransaction(indradb::Error),

    #[error("error while creating node: {0}.")]
    CreateNode(indradb::Error),
    #[error("error while setting node properties: {0}.")]
    SetNodeProperties(indradb::Error),
    #[error("error while getting vertices: {0}.")]
    GetNodes(indradb::Error),
    #[error("error while getting edges of a node: {0}.")]
    GetEdgesOfNodes(indradb::Error),
    #[error("error while updating node: {0}.")]
    UpdateNode(indradb::Error),
    #[error("error while deleting node: {0}.")]
    DeleteNode(indradb::Error),

    #[error("Custom type name is invalid.")]
    CreateType(indradb::ValidationError),
    #[error("error while parsing uuid: {0}.")]
    InvalidId(uuid::Error),

    #[error("error while creating edge: {0}.")]
    CreateEdge(indradb::Error),
    #[error("error while setting edge properties: {0}.")]
    SetEdgeProperties(indradb::Error),
    #[error("failed to create the edge.")]
    CreateEdgeFailed,
    #[error("error, could not delete outbound edges: {0}.")]
    DeleteOutboundEdges(indradb::Error),
    #[error("error, could not read edge properties: {0}.")]
    GetEdgeProperties(indradb::Error),
    #[error("error, could not delete inbound edges: {0}.")]
    DeleteInboundEdges(indradb::Error),
    #[error("error, could not update edge properties: {0}.")]
    UpdateEdgeProperties(indradb::Error),
    #[error("error, could not delete edge: {0}.")]
    DeleteEdge(indradb::Error),

    #[error("error, can't undo when buffer is empty.")]
    UndoBufferEmpty,
}

impl From<uuid::Error> for Error {
    fn from(error: uuid::Error) -> Error {
        Error::InvalidId(error)
    }
}

pub type Result<T> = StdResult<T, Error>;
