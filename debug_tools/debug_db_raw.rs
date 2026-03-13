use sled;

#[test]
fn test_inspect_tree_raw() {
    let db = sled::open(".something/db").unwrap();
    let tree_hash = "56851dcff88db93a8247b86af9ca0371ec777d3c424ad9cee8481aea9cd8bf2b";
    let data = db.get(tree_hash).unwrap().unwrap();
    println!("RAW_TREE_JSON:{}", String::from_utf8_lossy(&data));
}
