use sunshine_indra::{Config, CreateVertex, Error, Msg, Result, Store};

fn main() {
    let cfg = Config {
        db_path: "newdb".into(),
    };
    let store = Store::new(&cfg).unwrap();
    let id = store
        .execute(Msg::CreateVertex(CreateVertex {
            data: serde_json::json!({
                "name": "first_vertex",
            }),
        }))
        .unwrap();

    println!("{}", id);
}

// create edge, with properties
// editing existing edges
// read config from command line
// implement message as trait
