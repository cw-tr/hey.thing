use hey_thing::storage::kv_store::KvStore;
use hey_thing::core::sync::compute_delta;

#[test]
fn test_inspect_delta_json() {
    let store = KvStore::open(".something/db").unwrap();
    let head = "90f1e193b7d280985936496bba076c99e10a6eae109af65837f1c05b72e41077";
    match compute_delta(&store, head, None) {
        Ok(delta) => {
            let json = serde_json::to_string(&delta).unwrap();
            println!("DEBUG_DELTA_JSON:{}", json);
        },
        Err(e) => {
            println!("DELTA_ERROR: {:?}", e);
        }
    }
}
