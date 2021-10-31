pub mod error;
pub mod msg;
pub mod store;

use msg::{EdgeInfo, GraphId, Msg, ReadOnly, VertexInfo};
use store::Store;

pub struct UiStore {
    pub inner: Store,
    pub current_graph_id: GraphId,
    pub view: View,
}

pub struct View {
    pub root: VertexInfo,
    pub edges: Vec<EdgeInfo>,
    pub vertices: Vec<VertexInfo>,
}

impl UiStore {
    pub fn new(_cfg: &Config) -> UiStore {
        todo!()
    }

    pub fn send_msg(&self, _msg: Msg) {
        todo!()
    }

    pub fn update_view(&mut self) {
        let graph = self
            .inner
            .execute(Msg::ReadOnly(ReadOnly::ReadGraph(
                self.current_graph_id.clone(),
            )))
            .into_graph()
            .unwrap();
        let root = self
            .inner
            .execute(Msg::ReadOnly(ReadOnly::ReadVertex(
                self.current_graph_id.clone(),
            )))
            .into_vertex_info()
            .unwrap();
        let edges = graph
            .vertices
            .iter()
            .map(|vert| vert.outbound_edges.iter())
            .flatten()
            .map(|edge_id| {
                self.inner
                    .execute(Msg::ReadOnly(ReadOnly::ReadEdge(edge_id.clone())))
                    .into_edge_info()
                    .unwrap()
            })
            .collect();
        self.view = View {
            vertices: graph.vertices,
            root,
            edges,
        };
    }
}

pub struct Config {}
