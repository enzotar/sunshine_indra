pub mod store;

// use msg::{EdgeInfo, GraphId, Msg, MutateState, MutateStateKind, Node, Query};
// use store::Store;

// pub struct UiStore {
//     inner: Store,
//     current_graph_id: GraphId,
//     view: View,
// }

// pub struct View {
//     root: Node,
//     edges: Vec<EdgeInfo>,
//     vertices: Vec<Node>,
// }

// impl UiStore {
//     pub fn new() -> UiStore {
//         todo!()
//     }

//     fn send_msg(&self, msg: Msg) {
//         todo!()
//     }

//     fn update_view(&mut self) {
//         let graph = self
//             .inner
//             .execute(Msg::Query(Query::ReadGraph(self.current_graph_id.clone())))
//             .into_graph()
//             .unwrap();
//         let root = self
//             .inner
//             .execute(Msg::Query(Query::ReadNode(self.current_graph_id.clone())))
//             .into_node_info()
//             .unwrap();
//         let edges = graph
//             .vertices
//             .iter()
//             .map(|vert| vert.outbound_edges.iter())
//             .flatten()
//             .map(|edge_id| {
//                 self.inner
//                     .execute(Msg::Query(Query::ReadEdge(edge_id.clone())))
//                     .into_edge_info()
//                     .unwrap()
//             })
//             .collect();
//         self.view = View {
//             vertices: graph.vertices,
//             root: root.clone(),
//             edges,
//         };
//     }
// }
