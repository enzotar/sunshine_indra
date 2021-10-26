use sunshine_indra::{Config, CreateEdge, CreateVertex, Msg, Reply, Store};

fn main() {
    let cfg = Config {
        db_path: "newdb".into(),
    };
    let store = Store::new(&cfg).unwrap();

    let create_vertex = |properties: serde_json::Value| {
        let reply = store.execute(Msg::CreateVertex(CreateVertex {
            properties,
            vertex_type: "vertex_type1".into(),
        }));
        match reply {
            Reply::Id(id) => id,
            e => panic!("failed to create vertex: {:?}", e),
        }
    };

    let id1 = create_vertex(serde_json::json!({
        "name": "first_edge",
    }));

    println!("{}", id1);

    let reply = store.execute(Msg::UpdateVertex((
        id1.clone(),
        serde_json::json!({
            "name": "updated_first_vertex",
        }),
    )));

    println!("{:#?}", reply);

    let id2 = create_vertex(serde_json::json!({
        "name": "second_edge",
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

    let reply = store.execute(Msg::ReadVertex(id1));

    println!("{:#?}", reply);
}

// TODOs
// clear database
// read config from command line
// unit tests
// ui initialization
// cloud sync
//

// optimization
// clones and Strings

// Basic search
//  1. given a root node, get all children
//     a. breath-first
//     b. depth-first 3 levels
//  2. query with multiple hops/pipe edge query

// Multiplayer
// what libraries exists
//   CRDT?
// optimal
// quick and dirty
//
// how other projects implement it
// pijul
// xi-editor

// Learn more about Indra
// Pipe queries
// https://crates.io/crates/indradb-proto
// github/ozgrakkurt
