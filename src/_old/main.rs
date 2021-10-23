// https://docs.rs/lazy_static/1.4.0/lazy_static/
// https://docs.rs/indradb-lib/2.1.0/indradb/index.html
//

use indradb::{
    Datastore, EdgeKey, MemoryDatastore, SpecificVertexQuery, Transaction, Type, Vertex,
};
use serde_json::json;

mod data;

use lazy_static::lazy_static;
lazy_static! {
    static ref CONTACT_VERTEX: Type = Type::new("contact_vertex").expect("creating vertex type");
}

fn create_vertex(datastore: &MemoryDatastore, vertex_properties: &serde_json::Value) -> Vertex {
    let transaction = datastore.transaction().expect("Creating transaction");
    let contact_vertex = Vertex::new(CONTACT_VERTEX.clone());

    let created = transaction
        .create_vertex(&contact_vertex)
        .expect("Creating vertex");

    assert!(created, "Failed to add vertex to datastore");
    transaction
        .set_vertex_properties(
            indradb::VertexPropertyQuery::new(
                SpecificVertexQuery::single(contact_vertex.id).into(),
                String::from("properties"),
            ),
            vertex_properties,
        )
        .expect("setting vertex properties");

    contact_vertex
}

fn create_datastore() {}

fn main() {
    // data::create_transaction();

    let vertex_type = indradb::Type::new("type1").unwrap();

    let mem = MemoryDatastore::create("temp").expect("err");

    let transaction = mem.transaction().expect("starting transaction");

    let vertex1 = Vertex::new(vertex_type.clone());
    let vertex2 = Vertex::new(vertex_type);

    transaction
        .create_vertex(&vertex1)
        .expect("Creating vertex 1");
    transaction
        .create_vertex(&vertex2)
        .expect("Creating vertex 2");
    transaction
        .set_vertex_properties(
            indradb::VertexPropertyQuery::new(
                SpecificVertexQuery::single(vertex1.id).into(),
                String::from("contact_info"),
            ),
            &json!({
                "name": "John Doe",
                "age": 43,
                "phones": [
                    "+44 1234567",
                    "+44 2345678"
                ]
            }),
        )
        .expect("setting vertex properties");

    let etype = Type::new("edge_type").unwrap();
    let edge_key = EdgeKey::new(vertex1.id, etype, vertex2.id);

    transaction.create_edge(&edge_key).expect("Creating edge");
}
