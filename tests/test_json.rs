use serde_json;
use hey_thing::core::object_model::Tree;

#[test]
fn test_tree_json() {
    let json = r#"{"entries":[{"name":"./src/core/mod.rs","hash":"cb308dca002a2a8f171b067fb69deaefea9661b9de3c08323f8213a552e385a7","entry_type":"Blob","mode":33188,"is_chunked":false,"chunks":null}]}"#;
    let _: Tree = serde_json::from_str(json).unwrap();
}
