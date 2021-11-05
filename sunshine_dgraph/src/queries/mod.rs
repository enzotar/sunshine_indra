pub fn query_by_uid(uid: &str) -> String {
    format!(
        r#"{{
                find(func: uid({}))  @recurse{{
                    uid
                    name
                    display
                    inlineDisplay
                    validation
                    action
                    link
                    options
                    selectionMode
                }}
            }}"#,
        uid
    )
}





/*
 {
                "set":{
                  "uid":"$parentUid",
                  "link":{
                  "name":"$newNodeName",
                  "display":"$display",
                  "inlineDisplay":"$inlineDisplay",
                  "action":"$action"
                  }
                }
              }

*/
