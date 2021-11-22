// use sunshine_core::msg::{
//     Action, CreateEdge, Edge, EdgeId, Graph, GraphId, MutateKind, Node, NodeId, Properties,
//     RecreateNode,
// };
// // use sunshine_core::store::Datastore;
// use sunshine_indra::store::{DbConfig, DB};

// #[tokio::main]
// async fn main() {
//     let mut indra_store = DB::new(&DbConfig {
//         db_path: "indra_datastore".into(),
//     })
//     .unwrap();

//     let history = [CreateGraph, CreateNode, CreateEdge 6];

//     for local_msg in history.iter() {
//         let mut found = false;
//         for cloud_msg in cloud_history.iter() {
//             if local_msg == cloud_msg {
//                 found = true;
//                 break;
//             }
//         }

//         if !found {
//             cloud_db.execute(msg1);
//         }
//     }

//     // load graph
//     // work on it, add 100 actions
//     // connect to internet
// }

// // local: A, B, C

// // cloud: A B C

// struct State {
//     indra_store: DB,
//     cloud_store: CloudStore,
// }

// impl State {
//     pub fn new() -> Self {
//         let mut indra_store = DB::new(&DbConfig {
//             db_path: "indra_datastore".into(),
//         })
//         .unwrap();
//         State { indra_store }
//     }
// }

// struct CloudStore {}
