use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::str::FromStr;
use sunshine_core::store::Datastore;
use uuid::Uuid;

use crate::queries::*;
use crate::response::{Node as DNode, QueryRoot, UpsertRoot};

use sunshine_core::error::*;
use sunshine_core::msg::*;

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

#[async_trait]
impl Datastore for Store {
    fn undo_buf(&mut self) -> &mut Vec<Action> {
        &mut self.undo
    }

    fn redo_buf(&mut self) -> &mut Vec<Action> {
        &mut self.redo
    }

    fn history_buf(&mut self) -> &mut Vec<Action> {
        &mut self.history
    }

    async fn update_state_id(&self, graph_id: GraphId) -> Result<()> {
        let res: UpsertRoot = self
            .json_req(
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

        if res.data.queries.get("q").unwrap().len() < 1 {
            return Err(Error::GraphNotFound);
        }

        Ok(())
    }

    async fn create_graph(&self, _: Properties) -> Result<(Action, GraphId)> {
        Err(Error::Unimplemented)
    }

    async fn create_graph_with_id(
        &self,
        graph_id: GraphId,
        properties: Properties,
    ) -> Result<(Action, GraphId)> {
        let create_graph = Mutate {
            set: MutateCreateGraph {
                indra_id: graph_id.to_string(),
                is_graph_root: true,
                state_id: 0,
                properties: properties,
            },
        };

        self.json_req(MUTATE, &create_graph).await?;

        Ok((Action::DeleteGraph(graph_id), graph_id))
    }

    async fn list_graphs(&self) -> Result<Vec<(NodeId, Properties)>> {
        let res: QueryRoot = self
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

        res.data
            .get("q")
            .unwrap()
            .into_iter()
            .map(|node| Ok((Uuid::from_str(&node.indra_id)?, node.properties.clone())))
            .collect::<Result<Vec<_>>>()
    }

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        let res: QueryRoot = self
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

        if res.data.get("q").unwrap().len() < 1 {
            return Err(Error::GraphNotFound);
        }

        let node = &res.data.get("q").unwrap()[0];

        let nodes = match node.link.as_ref() {
            Some(nodes) => todo!(), //nodes.iter().map(|node| Node {}),
            None => Vec::new(),
        };

        Ok(Graph {
            state_id: node.properties.get("state_id").unwrap().as_u64().unwrap(),
            nodes,
        })
    }

    async fn create_node_with_id(
        &self,
        indra_id: NodeId,
        (graph_id, properties): (GraphId, Properties),
    ) -> Result<Action> {
        let res: UpsertRoot = self
            .json_req(
                MUTATE,
                &serde_json::json!({
                    "query": format!(r#"{{
                        q(func: eq(indra_id,"{}")) {{
                        u as uid
                        indra_id
                    }}
            }}"#, graph_id),
                    "set": {
                        "uid": "uid(u)",
                        "link": MutateCreateNode {
                            indra_id: indra_id.to_string(),
                            properties,
                        }
                    },
                }),
            )
            .await?;

        // a -> b -> c -> a
        //   b -> a

        // graph_root { a { b }, b { c, a }, c { a } }

        if res.data.queries.get("q").unwrap().len() < 1 {
            return Err(Error::GraphNotFound);
        }

        Ok(Action::Mutate(graph_id, MutateKind::DeleteNode(indra_id)))
    }

    async fn read_node(&self, node_id: NodeId) -> Result<Node> {
        let res: QueryRoot = self
            .dql_req(
                QUERY,
                format!(
                    "{{
                outbound(func: eq(indra_id, \"{}\")) {{
                    uid
                    indra_id
                    link {{
                        uid
                        indra_id
                    }}
                }}

                inbound(func: eq(indra_id, \"{}\")) {{
                    uid
                    indra_id
                    ~link {{
                        uid
                        indra_id
                    }}
                }}
            }}",
                    node_id, node_id
                ),
            )
            .await?;

        let outbound = res.data.get("outbound").unwrap();
        let inbound = res.data.get("inbound").unwrap();

        if outbound.len() < 1 {
            return Err(Error::NodeNotFound);
        }

        let outbound = &outbound[0];

        if inbound.len() < 1 {
            return Err(Error::NodeNotFound);
        }

        let inbound = &inbound[0];

        let outbound_edges: Vec<Edge> = match outbound.link.as_ref() {
            Some(nodes) => todo!(),
            None => Vec::new(),
        };

        let inbound_edges: Vec<Edge> = match inbound.link.as_ref() {
            Some(nodes) => todo!(),
            None => Vec::new(),
        };

        Ok(Node {
            node_id,
            properties: outbound.properties.clone(),
            outbound_edges: Vec::new(),
            inbound_edges: Vec::new(),
        })
    }

    async fn update_node(&self, args: (NodeId, Properties), graph_id: GraphId) -> Result<Action> {
        todo!();
    }

    async fn recreate_node(
        &self,
        recreate_node: RecreateNode,
        graph_id: GraphId,
    ) -> Result<Action> {
        todo!();
    }

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Action> {
        todo!();
    }

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Action, EdgeId)> {
        todo!();
    }

    async fn read_edge_properties(&self, msg: Edge) -> Result<Properties> {
        todo!();
    }
    async fn recreate_edge(&self, edge: Edge, properties: Properties) -> Result<()> {
        todo!();
    }

    async fn update_edge(
        &self,
        (edge, properties): (Edge, Properties),
        graph_id: GraphId,
    ) -> Result<Action> {
        todo!();
    }

    async fn delete_edge(&self, edge: Edge, graph_id: GraphId) -> Result<Action> {
        todo!();
    }
}

struct Store {
    undo: Vec<Action>,
    redo: Vec<Action>,
    history: Vec<Action>,
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

    async fn json_req<B: Serialize, T: DeserializeOwned>(
        &self,
        url_part: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.base_url.to_owned() + url_part;

        let res = self
            .client
            .post(url)
            .header("x-auth-token", &self.auth_token)
            .json(body)
            .send()
            .await
            .map_err(Error::HttpClientError)?;

        Self::parse_response(res).await
    }

    async fn dql_req<S: Into<String>, T: DeserializeOwned>(
        &self,
        url_part: &str,
        body: S,
    ) -> Result<T> {
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

    async fn parse_response<T: DeserializeOwned>(res: reqwest::Response) -> Result<T> {
        let json = Self::check_err_response(res).await?;

        println!("{:#?}", json);

        serde_json::from_value(json).map_err(Error::JsonError)
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
    use sunshine_core::store::Datastore;

    fn make_store() -> StoreImpl {
        StoreImpl::new(&Config {
            base_url: "https://quiet-leaf.us-west-2.aws.cloud.dgraph.io".into(),
            auth_token: "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=".into(),
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_state_id() {
        make_store()
            .update_state_id(Uuid::from_str("0d0bd4ee-40f0-11ec-973a-0242ac130003").unwrap())
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
    async fn test_read_graph() {
        dbg!(
            make_store()
                .read_graph(Uuid::from_str("2ac209c6-40ce-11ec-9884-8b4b20e8c2eb").unwrap())
                .await
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_node_with_id() {
        let properties = json!({
            "name":"first Node",
            "age": 13
        });
        let properties = match properties {
            JsonValue::Object(props) => props,
            _ => unreachable!(),
        };

        dbg!(
            make_store()
                .create_node_with_id(
                    Uuid::from_str("5dd79972-4329-11ec-81d3-0242ac130003").unwrap(),
                    (
                        Uuid::from_str("0d0bd4ee-40f0-11ec-973a-0242ac130003").unwrap(),
                        properties
                    )
                )
                .await
        );
    }
}

// 0xfffd8d6aac73f42d
// 2ac209c6-40ce-11ec-9884-8b4b20e8c2eb
//0d0bd4ee-40f0-11ec-973a-0242ac130003

// history: CreateNode, UpdateNode ....

// history: CreateNode
