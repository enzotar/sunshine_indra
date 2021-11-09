// use serde::{Deserialize, Serialize};
// use serde_json::Value as JsonValue;

// pub type FlatMap = serde_json::Map<String, NonObjectJsonValue>;

// // {
// //     "set":{
// //         "indra_id":....,
// //         "state_id": "0"
// //     }
// // }

// #[derive(Serialize, Deserialize)]
// pub struct NonObjectJsonValue(JsonValue);

// #[derive(Serialize)]
// pub enum NonObjectJsonValue {
//     Number(u32),
// }

// impl NonObjectJsonValue {
//     pub fn new(val: JsonValue) -> Option<Self> {
//         match val {
//             JsonValue::Object(_) => None,
//             _ => Some(NonObjectJsonValue(val)),
//         }
//     }
// }

// user need to pass properties as hashmap
// creates msgs with properties
// added to cloud queue
// process queue with converted properties
