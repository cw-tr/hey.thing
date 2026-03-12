#[cfg(test)]
mod tests {
    use crate::core::object_model::Commit;
    use crate::crypto::hash::hash_data;

    #[test]
    fn test_hash_consistency() {
        let data = b"hey thing content";
        let h1 = hash_data(data);
        let h2 = hash_data(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_commit_integrity() {
        let commit = Commit {
            parent_id: None,
            tree_hash: "abcd".to_string(),
            author: "Tester".to_string(),
            timestamp: 123456789,
            message: "Test commit".to_string(),
        };
        let json = serde_json::to_vec(&commit).unwrap();
        let hash = hash_data(&json);
        assert!(commit.verify_integrity(&hash));
    }
}
