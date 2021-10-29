// use sunshine_indra::{Config, CreateEdge, CreateVertex, Msg, Reply, Store};

fn main() {
    // let cfg = Config {
    //     db_path: "newdb".into(),
    // };
    // let store = Store::new(&cfg).unwrap();

    // let create_vertex = |properties: serde_json::Value| {
    //     let reply = store.execute(Msg::CreateVertex(CreateVertex {
    //         properties,
    //         vertex_type: "vertex_type1".into(),
    //     }));
    //     match reply {
    //         Reply::Id(id) => id,
    //         e => panic!("failed to create vertex: {:?}", e),
    //     }
    // };

    // let id1 = create_vertex(serde_json::json!({
    //     "name": "first_edge",
    // }));

    // println!("{}", id1);

    // let reply = store.execute(Msg::UpdateVertex((
    //     id1.clone(),
    //     serde_json::json!({
    //         "name": "updated_first_vertex",
    //     }),
    // )));

    // println!("{:#?}", reply);

    // let id2 = create_vertex(serde_json::json!({
    //     "name": "second_edge",
    // }));

    // println!("{}", id2);

    // let reply = store.execute(Msg::CreateEdge(CreateEdge {
    //     directed: false,
    //     from: id1.clone(),
    //     edge_type: "edge_type1".into(),
    //     to: id2,
    //     properties: serde_json::json!({
    //         "name": "first_edge",
    //     }),
    // }));

    // println!("{:#?}", reply);

    // let reply = store.execute(Msg::ReadVertex(id1));

    // println!("{:#?}", reply);
}

// TODOs
// undo: Vec<Msg>, redo: Vec<Msg>
// delete graph
// improve msg api so it is clear if we need to update state_id

// clear database/ init
// read config from command line
// unit tests
// ui initialization
// cloud sync

//

// Basic search
// https://crates.io/crates/pathfinding
//  1. given a root node, get all children
//     a. breath-first
//     b. depth-first , e.g. 3 levels
//     maybe example of algorithm https://crates.io/crates/graphsearch
//  2. query with multiple hops/pipe edge query

// optimization
// clones and Strings

// Multiplayer
// what libraries exists
//   CRDT? message queue?
// optimal way to implement?
// quick and dirty?
// https://hex.tech/blog/a-pragmatic-approach-to-live-collaboration
// https://www.figma.com/blog/how-figmas-multiplayer-technology-works/
//
// https://github.com/davidrusu/bft-crdts
// https://github.com/automerge/automerge-rs
// https://wiki.nikitavoloboev.xyz/distributed-systems/crdt
//
// how other projects implement it
// pijul
// xi-editor

// Learn more about Indra
// Pipe queries
// https://crates.io/crates/indradb-proto
// github/ozgrakkurt

// Use types for namespacing. For seperating projects, versions of the same project and for unit testing.
// Every document has a counter and messages are sent with a counter value, if the count is lower than count on the current document update is rejected.
// We keep last X number of messages in a buffer so we can do undo operations. Every message should have a reverse type.

// open project1.graph
// type: project1 vertex, edges
//
// root node  : project1
//  children nodes

// undo/redo - do you delete or do you make a new state without?

// saved_ops: HashMap<"name of op", Vec<Msg, Msg, Msg,>>,

// timemachine slider
// record every activity in timeseries db
// templating
// node, edge, node
// ...
// back in time, select node, edge, node, paste

// create vertex
// undo delete vertex / add redo
// copy/paste
// template

// graph state using unique id uuid v4
// hash == E-Tag Arangodb

// load file, check specialhash, if equal, got version
