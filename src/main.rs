use sunshine_indra::{Config, CreateEdge, CreateVertex, Error, Msg, Reply, Result, Store};

fn main() {
    let cfg = Config {
        db_path: "newdb".into(),
    };
    let store = Store::new(&cfg).unwrap();

    let create_vertex = |properties: serde_json::Value| {
        let reply = store.execute(Msg::CreateVertex(CreateVertex {
            properties: serde_json::json!({
                "name": "first_vertex",
            }),
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

// editing existing edges
// read config from command line
// tidy UUID in create edges
// create edge better error display
// thiserror

// delete
