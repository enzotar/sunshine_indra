use serde::{Deserialize, Serialize};
use sunshine_core::msg::Properties;

#[derive(Serialize, Debug)]
pub struct Mutate<T: Serialize> {
    pub set: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MutateCreateGraph {
    pub indra_id: String,
    pub state_id: i32,
    pub is_graph_root: bool,
    #[serde(flatten)]
    pub properties: Properties,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MutateCreateNode {
    pub indra_id: String,
    #[serde(flatten)]
    pub properties: Properties,
}

// pub struct Upsert {
//     pub query: Query
//     pub set:,
// }

// pub struct Query {
//     pub query: String,
// }

// pub struct MutateNode{
//     pub uid
// }

// read_node
// """
// find(func: uid("{node_id}"))  {
//                     uid
//                     link{
//                       uid
//                     }
//                     }
// """

// createNode

//       """{
//                 "set":{
//                   "uid":"$parentUid",
//                   "link":{
//                   "name":"$newNodeName",
//                   "node_type:

//                   }
//                 }
//               }
//               """;

// CreateNode no parent

//         /*
//         {
//                 "set":{
//                   "indra_id":"{}",
//                   "state_id":"0"
//                 }
//               }
//                */
// Upsert Example with math function
// upsert {{
//                     query {{
//                         q(func: eq(indra_id,"{}")) {{
//                         u as uid
//                         s as state_id
//                         n as math(s+1)
//                         indra_id
//                         }}
//                     }}

//                     mutation {{
//                         set {{
//                             uid(u) <state_id> val(n).
//                         }}
//                     }}
//                 }}

// Query

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

// class GraphQueries {
//   static String cheetahNodeQuery() => """{
//       find(func: uid("0x2"))  @recurse{
//       uid
//       name
//       display
//       inlineDisplay
//       link
//       }
//     }""";

//   /// TODO Document what the purpose of this query is. "entries" is too general.
//   static String entries(String uid) => """{
//             find(func: uid("$uid"))  @recurse{
//               uid
//               name
//               display
//               inlineDisplay
//               validation
//               action
//               link
//               options
//               selectionMode
//               }
//             }
//               """;

//   static String userModel() => """{
//                         find(func: uid("0x2"))  @recurse{
//                           uid
//                         name
//                         display
//                         inlineDisplay
//                         action
//                         link
//                         }
//                       }
//                         """;

//   static String rootNodes(String rootNodeUid) => """{
//                   find(func: uid("$rootNodeUid"))  @recurse{
//                     uid
//                     name
//                     display
//                     inlineDisplay
//                     validation
//                     action
//                     link
//                     }
//                   }
//                     """;

//   static String mutateNode(String parentUid, String newNodeName) => """{
//                 "set":{
//                   "uid":"0x2711",
//                   "link":{
//                   "name":"$newNodeName"
//                   }
//                 }
//               }
//               """;

//   /// TODO This name is too general. Add what if what is absent?
//   static String addIfAbsent(String value) => """{
//                   find(func: eq(name, "$value"))  @recurse{
//                     uid
//                     name
//                     display
//                     inlineDisplay
//                     validation
//                     action
//                     link
//                     }
//                   }
//                     """;

//   static String fetchUserData(String uid) => """{
//                   find(func: eq(firebaseUid, "${uid}"))  {
//                     uid
//                     name
//                     link
//                     }
//                   }
//                     """;

//   static String fetchMyListings(String userUid) => """{
//                     find(func: uid("0xe")){
//                         uid
//                         link @facets(eq,"$userUid"){
//                         uid
//                         }
//                       }
//                     }
//                     """;

//   // Node 0xe is a node with name listing.
//   // Connect listing node to a new node with
//   static String createListing(String userUid) => """{
//                 "set":{
//                   "uid":"0xe",
//                   "link":[{
//                   "link|userId":"$userUid"
//                   }]
//                 }
//               }
//               """;

//   static String addEdge({
//     required String fromNode,
//     required String toNode,
//     required String facetId,
//     required String facetValue,
//   }) {
//     return """{
//                 "set":{
//                   "uid":"$fromNode",
//                   "link":[{
//                   "uid":"$toNode",
//                   "link|$facetId":"$facetValue"
//                   }]
//                 }
//               }
//              """;
//   }
// }
