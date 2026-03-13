use sled;
use serde_json;
use hey_thing::core::object_model::Tree;

#[test]
fn test_find_corrupt_tree() {
    let db = sled::open(".something/db").unwrap();
    let mut count = 0;
    for result in db.iter() {
        let (key, value) = result.unwrap();
        let key_str = String::from_utf8_lossy(&key);
        let val_str = String::from_utf8_lossy(&value);
        
        // Tree nesnesi mi kontrol et (basitçe JSON içeriğine bakarak)
        if val_str.contains("\"entries\"") {
            match serde_json::from_slice::<Tree>(&value) {
                Ok(_) => {},
                Err(e) => {
                    println!("CORRUPT_TREE: {} - Error: {}", key_str, e);
                    count += 1;
                }
            }
        }
    }
    println!("Total corrupt trees found: {}", count);
}
