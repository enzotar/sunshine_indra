use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use std::str::FromStr;
use uuid::Uuid;

mod queries;
mod responses;

use queries::*;
use responses::node::{Node as DNode, Root};

use sunshine_core::msg::{
    CreateEdge, Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId,
    Properties, Query, RecreateNode, Reply,
};

use sunshine_core::{Error, Result};
// #[tokio::main]
// pub async fn query() -> std::result::Result<DNode, Box<dyn std::error::Error>> {
//     let client = reqwest::Client::new();

//     let url = "https://quiet-leaf.us-west-2.aws.cloud.dgraph.io/query?=";
//     let uid = "0x170f16be";

//     let res = client
//         .post(url)
//         .body(query_by_uid(uid))
//         .header(
//             "x-auth-token",
//             "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=",
//         )
//         .header("Content-Type", "application/graphql+-")
//         .send()
//         .await?;

//     let t: Root = res.json().await?;

//     let root_node: &DNode = &t.data.find.first().unwrap();

//     dbg!(t.clone());

//     Ok(root_node.clone())
// }

const MUTATE: &str = "/mutate?commitNow=true";
const QUERY: &str = "/query";

#[async_trait::async_trait]
impl sunshine_core::Store for Store {
    fn undo_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.undo
    }

    fn redo_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.redo
    }

    fn history_buf(&mut self) -> &mut Vec<Msg> {
        &mut self.history
    }

    async fn update_state_id(&self, graph_id: GraphId) -> Result<()> {
        self.json_req(
            MUTATE,
            &serde_json::json!({
                "query": format!(r#"{{
                q(func: eq(indra_id,"{}")) {{
                u as uid
                s as state_id
                n as math(s+1)
                indra_id
                }}
            }}"#, graph_id),
                "set": {
                    "uid": "uid(u)",
                    "state_id": "val(n)",
                },
            }),
        )
        .await?;

        Ok(())
    }

    async fn create_graph(&self, _: Properties) -> Result<(Msg, GraphId)> {
        Err(Error::Unimplemented)
    }

    async fn create_graph_with_id(
        &self,
        graph_id: GraphId,
        properties: Properties,
    ) -> Result<(Msg, GraphId)> {
        let create_graph = Mutate {
            set: MutateCreateGraph {
                indra_id: graph_id.to_string(),
                is_graph_root: true,
                state_id: 0,
                properties: properties,
            },
        };

        self.json_req(MUTATE, &create_graph).await?;

        Ok((Msg::DeleteGraph(graph_id), graph_id))
    }

    async fn list_graphs(&self) -> Result<Vec<(NodeId, Properties)>> {
        let res = self
            .dql_req(
                QUERY,
                "{
                q(func: eq(is_graph_root,true)) {
                    uid
                    state_id
                    indra_id
                }
            }",
            )
            .await?;

        res.into_iter()
            .map(|node| Ok((Uuid::from_str(&node.indra_id)?, node.properties)))
            .collect::<Result<Vec<_>>>()
    }

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let res = self
            .dql_req(
                QUERY,
                format!(
                    "{{
                q(func: eq(indra_id, \"{}\")) @recurse{{
                    uid
                    indra_id
                    state_id
                    link
                }}
            }}",
                    graph_id
                ),
            )
            .await?;

        if res.len() < 1 {
            return Err(Error::GraphNotFound);
        }

        let node = &res[0];

        let nodes = match node.link.as_ref() {
            Some(nodes) => todo!(),
            None => Vec::new(),
        };

        Ok(Graph {
            state_id: node.properties.get("state_id").unwrap().as_u64().unwrap(),
            nodes,
        })
    }

    // https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/

    async fn create_node(
        &self,
        (graph_id, properties): (GraphId, Properties),
    ) -> Result<(Msg, NodeId)> {
        todo!();
    }

    async fn read_node(&self, node_id: NodeId) -> Result<Node> {
        todo!();
    }

    async fn update_node(
        &self,
        (node_id, value): (NodeId, Properties),
        graph_id: GraphId,
    ) -> Result<Msg> {
        todo!();
    }

    async fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn recreate_edge(&self, edge: Edge, properties: Properties) -> Result<()> {
        todo!();
    }

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Msg, EdgeId)> {
        todo!();
    }

    async fn read_edge_properties(&self, msg: Edge) -> Result<Properties> {
        todo!();
    }

    async fn update_edge(
        &self,
        (edge, properties): (Edge, Properties),
        graph_id: GraphId,
    ) -> Result<Msg> {
        todo!();
    }

    async fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }
}

struct Store {
    undo: Vec<Msg>,
    redo: Vec<Msg>,
    history: Vec<Msg>,
    client: reqwest::Client,
    base_url: String,
    auth_token: String,
}

impl Store {
    pub fn new(cfg: &Config) -> Store {
        let client = reqwest::Client::builder().build().unwrap();
        Store {
            undo: Vec::new(),
            redo: Vec::new(),
            history: Vec::new(),
            client,
            base_url: cfg.base_url.clone(),
            auth_token: cfg.auth_token.clone(),
        }
    }

    async fn json_req<B: Serialize>(&self, url_part: &str, body: &B) -> Result<()> {
        let url = self.base_url.to_owned() + url_part;

        let res = self
            .client
            .post(url)
            .header("x-auth-token", &self.auth_token)
            .json(body)
            .send()
            .await
            .map_err(Error::HttpClientError)?;

        Self::check_err_response(res).await?;

        Ok(())
    }

    async fn dql_req<S: Into<String>>(&self, url_part: &str, body: S) -> Result<Vec<DNode>> {
        let url = self.base_url.to_owned() + url_part;

        let res = self
            .client
            .post(url)
            .header("x-auth-token", &self.auth_token)
            .body(body.into())
            .header("content-type", "application/dql")
            .send()
            .await
            .map_err(Error::HttpClientError)?;

        Self::parse_response(res).await
    }

    async fn check_err_response(res: reqwest::Response) -> Result<JsonValue> {
        let json = res
            .json::<JsonValue>()
            .await
            .map_err(Error::HttpClientError)?;

        if json.as_object().unwrap().contains_key("errors") {
            let err = serde_json::to_string_pretty(&json).map_err(Error::JsonError)?;
            return Err(Error::DGraphError(err));
        }

        Ok(json)
    }

    async fn parse_response(res: reqwest::Response) -> Result<Vec<DNode>> {
        let json = Self::check_err_response(res).await?;

        println!("{:#?}", json);

        let root: Root = serde_json::from_value(json).map_err(Error::JsonError)?;

        Ok(root.data.q)
    }
}

pub struct Config {
    base_url: String,
    auth_token: String,
}

#[cfg(test)]
mod tests {
    use super::Store as StoreImpl;
    use super::*;
    use serde_json::json;
    use std::str::FromStr;
    use sunshine_core::Store;

    fn make_store() -> StoreImpl {
        StoreImpl::new(&Config {
            base_url: "https://quiet-leaf.us-west-2.aws.cloud.dgraph.io".into(),
            auth_token: "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=".into(),
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_state_id() {
        make_store()
            .update_state_id(Uuid::from_str("2ac209c6-40ce-11ec-9884-8b4b20e8c2eb").unwrap())
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_graph_with_id() {
        let store = make_store();
        let properties = json!({
            "name":"test",
            "cost":2800,
        });
        let properties = match properties {
            JsonValue::Object(props) => props,
            _ => unreachable!(),
        };
        store
            .create_graph_with_id(
                Uuid::from_str("0d0bd4ee-40f0-11ec-973a-0242ac130003").unwrap(),
                properties,
            )
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_graphs() {
        dbg!(make_store().list_graphs().await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn read_graph() {
        dbg!(
            make_store()
                .read_graph(Uuid::from_str("2ac209c6-40ce-11ec-9884-8b4b20e8c2eb").unwrap())
                .await
        );
    }
}

// 0xfffd8d6aac73f42d
// 2ac209c6-40ce-11ec-9884-8b4b20e8c2eb
//0d0bd4ee-40f0-11ec-973a-0242ac130003
