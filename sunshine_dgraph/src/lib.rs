use serde_json::Value as JsonValue;
use uuid::Uuid;

mod queries;
mod responses;

use queries::*;
use responses::node::{Node as DNode, Root};

use sunshine_core::msg::{
    CreateEdge, Edge, EdgeId, Graph, GraphId, Msg, MutateState, MutateStateKind, Node, NodeId,
    Query, RecreateNode, Reply,
};

use sunshine_core::{Error, Result};

#[tokio::main]
pub async fn query() -> std::result::Result<DNode, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // let query_by_uid = |uid: &str| {
    //     format!(
    //         r#"{{
    //             find(func: uid({}))  @recurse{{
    //                 uid
    //                 name
    //                 display
    //                 inlineDisplay
    //                 validation
    //                 action
    //                 link
    //                 options
    //                 selectionMode
    //             }}
    //         }}"#,
    //         uid
    //     )
    // };

    //mutate?commitNow=true

    let url = "https://quiet-leaf.us-west-2.aws.cloud.dgraph.io/query?=";
    let uid = "0x170f16be";

    let res = client
        .post(url)
        .body(query_by_uid(uid))
        .header(
            "x-auth-token",
            "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=",
        )
        .header("Content-Type", "application/graphql+-")
        .send()
        .await?;

    let t: Root = res.json().await?;

    let root_node: &DNode = &t.data.find.first().unwrap();

    dbg!(t.clone());
    // rid::log_debug!("Got query {:#?}", &root_node);

    Ok(root_node.clone())
}

/*
curl -H "Content-Type: application/rdf" -X POST localhost:8080/mutate?commitNow=true -d $'
upsert {
  query {
    q(func: eq(email, "user@company1.io")) {
      v as uid
      name
    }
  }

  mutation {
    set {
      uid(v) <name> "first last" .
      uid(v) <email> "user@company1.io" .
    }
  }
}' | jq
*/

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
        let url = self.base_url.to_owned() + MUTATE;

        let res = self
            .client
            .post(url)
            .body(format!(
                r#"upsert {{
                    query {{
                        q(func: eq(indra_id,"{}")) {{
                        u as uid
                        s as state_id
                        n as math(s+1)
                        indra_id
                        }}
                    }}
                    
                    mutation {{
                        set {{
                            uid(u) <state_id> val(n).
                        }}
                    }}
                }}
                "#,
                graph_id
            ))
            .header(
                "x-auth-token",
                "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=",
            )
            .header("Content-Type", "application/rdf")
            .send()
            .await
            .unwrap();

        let json = res
            .json::<JsonValue>()
            .await
            .map_err(Error::HttpClientError)?;

        if json.as_object().unwrap().contains_key("errors") {
            let err = serde_json::to_string_pretty(&json).map_err(Error::JsonError)?;
            return Err(Error::DGraphError(err));
        }

        Ok(())
    }

    async fn create_graph(&self, _: JsonValue) -> Result<(Msg, GraphId)> {
        Err(Error::Unimplemented)
    }

    async fn create_graph_with_id(
        &self,
        graph_id: GraphId,
        properties: JsonValue,
    ) -> Result<(Msg, GraphId)> {
        let url = self.base_url.to_owned() + MUTATE;

        let res = self
            .client
            .post(url)
            .body(format!(
                r#"{{
                    "set":{{
                      "indra_id":"{}",
                      "state_id":"0" // add properties
                    }}
                }}"#,
                graph_id
            ))
            .header(
                "x-auth-token",
                "NmY2YWQ1YzlkNjg4NjUwMzc0MDJmMjk4ZTg3Yzk5Yzc=",
            )
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        let json = res
            .json::<JsonValue>()
            .await
            .map_err(Error::HttpClientError)?;

        if json.as_object().unwrap().contains_key("errors") {
            let err = serde_json::to_string_pretty(&json).map_err(Error::JsonError)?;
            return Err(Error::DGraphError(err));
        }

        Ok((Msg::DeleteGraph(graph_id), graph_id))
    }

    async fn list_graphs(&self) -> Result<Vec<Node>> {
        todo!();
    }

    async fn read_graph(&self, graph_id: GraphId) -> Result<Graph> {
        todo!();
    }

    async fn create_node(
        &self,
        (graph_id, properties): (GraphId, JsonValue),
    ) -> Result<(Msg, NodeId)> {
        todo!();
    }

    async fn read_node(&self, node_id: NodeId) -> Result<Node> {
        todo!();
    }

    async fn update_node(
        &self,
        (node_id, value): (NodeId, JsonValue),
        graph_id: GraphId,
    ) -> Result<Msg> {
        todo!();
    }

    async fn recreate_node(&self, recreate_node: RecreateNode, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn recreate_edge(&self, edge: Edge, properties: JsonValue) -> Result<()> {
        todo!();
    }

    // deletes inbound and outbound edges as well
    async fn delete_node(&self, node_id: NodeId, graph_id: GraphId) -> Result<Msg> {
        todo!();
    }

    async fn create_edge(&self, msg: CreateEdge, graph_id: GraphId) -> Result<(Msg, EdgeId)> {
        todo!();
    }

    async fn read_edge_properties(&self, msg: Edge) -> Result<JsonValue> {
        todo!();
    }

    async fn update_edge(
        &self,
        (edge, properties): (Edge, JsonValue),
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
}

impl Store {
    pub fn new<S: Into<String>>(base_url: S) -> Store {
        let client = reqwest::Client::builder().build().unwrap();
        Store {
            undo: Vec::new(),
            redo: Vec::new(),
            history: Vec::new(),
            client,
            base_url: base_url.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Store as StoreImpl;
    use super::*;
    use std::str::FromStr;
    use sunshine_core::Store;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_state_id() {
        let store = StoreImpl::new("https://quiet-leaf.us-west-2.aws.cloud.dgraph.io");
        store
            .update_state_id(Uuid::from_str("2ac209c6-40ce-11ec-9884-8b4b20e8c2eb").unwrap())
            .await
            .unwrap();
    }
}

// 0xfffd8d6aac73f42d
// 2ac209c6-40ce-11ec-9884-8b4b20e8c2eb
