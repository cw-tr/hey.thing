use hey_thing::storage::kv_store::KvStore;

#[test]
fn test_inspect_tree() {
    let store = KvStore::open(".something/db").unwrap();
    let tree_hash = "56851dcff88db93a8247b86af9ca0371ec777d3c424ad9cee8481aea9cd8bf2b";
    let data = store.get(tree_hash.as_bytes()).unwrap().unwrap();
    println!("DEBUG_TREE_JSON:{}", String::from_utf8_lossy(&data));
}
