// If you're coming from a javascript background and using something like json to take in method calls from somewhere else, you can do something like this for a fixed number of functions that you would define in the enum. This way though, you would have to predefine the functions that you allow to be called.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "fname", rename_all = "lowercase")]
enum Method {
    Add { a: u64, b: u64 },
}

impl Method {
    fn call(&self) -> serde_json::Value {
        match self {
            Method::Add { a, b } => serde_json::json!(a + b),
        }
    }
}

fn main() {
    let method_str = r#"{"fname":"add","a":1,"b":2}"#;
    let method: Method = serde_json::from_str(method_str).unwrap();
    let result = method.call();
    println!("{:?}", result);
}
// The serde_json::Value type is the trick needed to be able to take a variety of value types as a return since it can encapsulate numbers, strings, arrays, maps, etc.
