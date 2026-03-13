use sled;
use serde_json;
use serde_json::Value;

#[test]
fn test_fix_corrupt_v1() {
    let db = sled::open(".something/db").unwrap();
    let mut fixed = 0;
    for result in db.iter() {
        let (key, value) = result.unwrap();
        let val_str = String::from_utf8_lossy(&value);
        
        if val_str.contains("\"entries\"") {
            let mut v: Value = serde_json::from_slice(&value).unwrap();
            let mut changed = false;
            if let Some(entries) = v.get_mut("entries").and_then(|e| e.as_array_mut()) {
                for entry in entries {
                    if entry.get("is_chunked").is_none() {
                        entry.as_object_mut().unwrap().insert("is_chunked".to_string(), Value::Bool(false));
                        changed = true;
                    }
                    if entry.get("chunks").is_none() {
                        entry.as_object_mut().unwrap().insert("chunks".to_string(), Value::Null);
                        changed = true;
                    }
                }
            }
            if changed {
                let new_val = serde_json::to_vec(&v).unwrap();
                db.insert(&key, new_val).unwrap();
                fixed += 1;
            }
        }
    }
    db.flush().unwrap();
    println!("Fixed {} corrupt objects.", fixed);
}
