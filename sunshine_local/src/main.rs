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
//     indra_store
//         .execute(Action::CreateGraph(serde_json::Map::new()))
//         .await
//         .unwrap();
// }

// struct State {
//     indra_store: DB,
//     cloud_handle: Option<CloudHandle>,
// }

// struct CloudHandle {
//     client: reqwest::Client,
// }

// impl CloudHandle {
//     fn new(url: String, history: &[Action]) {
//         todo!()
//     }

//     fn execute(&self, action: Action) -> () {
//         todo!()
//     }
// }

fn main() {}
