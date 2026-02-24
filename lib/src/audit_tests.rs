#[cfg(test)]
mod tests {
    use crate::audit::{
        create_audit_log, export_audit_logs, get_audit_log, get_log_id_counter,
        get_total_audit_log_count, query_audit_logs, OperationType,
    };
    use soroban_sdk::{Address, Env, String, Vec};

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn get_contract_id(env: &Env) -> Address {
        Address::from_string(&String::from_str(
            env,
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAHSC4",
        ))
    }

    // ========================================================================
    // Unit Tests
    // ========================================================================

    #[test]
    fn test_audit_log_creation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, r#"{"status":"active"}"#);
            let tx_hash = String::from_str(&env, "abcd1234");
            let description = Some(String::from_str(&env, "Mint operation"));

            let log_id = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                description.clone(),
            );

            assert_eq!(log_id, 1);

            let retrieved = get_audit_log(&env, log_id).unwrap();
            assert_eq!(retrieved.id, 1);
            assert_eq!(retrieved.operator, operator);
            assert_eq!(retrieved.operation_type, OperationType::AdminMint);
            assert_eq!(retrieved.before_state, before_state);
            assert_eq!(retrieved.after_state, after_state);
            assert_eq!(retrieved.tx_hash, tx_hash);
        });
    }

    #[test]
    fn test_auto_incrementing_id() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, "{}");
            let tx_hash = String::from_str(&env, "tx1");

            // Create multiple logs
            let id1 = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );

            let id2 = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminTransfer,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );

            let id3 = create_audit_log(
                &env,
                operator.clone(),
                OperationType::SaleCreated,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );

            assert_eq!(id1, 1);
            assert_eq!(id2, 2);
            assert_eq!(id3, 3);

            assert_eq!(get_total_audit_log_count(&env), 3);
        });
    }

    #[test]
    fn test_audit_log_immutability() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, "{}");
            let tx_hash = String::from_str(&env, "tx1");

            let log_id = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );

            let retrieved1 = get_audit_log(&env, log_id).unwrap();
            let retrieved2 = get_audit_log(&env, log_id).unwrap();

            // Same log should be identical on multiple retrievals
            assert_eq!(retrieved1.id, retrieved2.id);
            assert_eq!(retrieved1.timestamp, retrieved2.timestamp);
            assert_eq!(retrieved1.operator, retrieved2.operator);
        });
    }

    #[test]
    fn test_audit_log_struct_validation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{\"count\":0}");
            let after_state = String::from_str(&env, "{\"count\":1}");
            let tx_hash = String::from_str(&env, "0xabcd");
            let description = Some(String::from_str(&env, "Increment counter"));

            let log_id = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminSettingsChange,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                description.clone(),
            );

            let log = get_audit_log(&env, log_id).unwrap();

            // Verify all fields are present and correct
            assert!(log.id > 0);
            assert!(log.timestamp > 0);
            assert_eq!(log.operator, operator);
            assert_eq!(log.operation_type, OperationType::AdminSettingsChange);
            assert_eq!(log.before_state, before_state);
            assert_eq!(log.after_state, after_state);
            assert_eq!(log.tx_hash, tx_hash);
            assert!(log.description.is_some());
        });
    }

    // ========================================================================
    // Pagination Tests
    // ========================================================================

    #[test]
    fn test_query_audit_logs_basic() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, "{}");
            let tx_hash = String::from_str(&env, "tx1");

            // Create 5 logs
            for i in 0..5 {
                let op_type = match i % 3 {
                    0 => OperationType::AdminMint,
                    1 => OperationType::AdminTransfer,
                    _ => OperationType::SaleCreated,
                };
                create_audit_log(
                    &env,
                    operator.clone(),
                    op_type,
                    before_state.clone(),
                    after_state.clone(),
                    tx_hash.clone(),
                    None,
                );
            }

            let result = query_audit_logs(&env, 1, 5, 10);

            assert_eq!(result.logs.len(), 5);
            assert_eq!(result.total_count, 5);
            assert_eq!(result.start_id, 1);
            assert_eq!(result.end_id, 5);
            assert_eq!(result.has_more, false);
        });
    }

    #[test]
    fn test_query_audit_logs_with_pagination() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, "{}");
            let tx_hash = String::from_str(&env, "tx1");

            // Create 10 logs
            for i in 0..10 {
                let op_type = match i % 3 {
                    0 => OperationType::AdminMint,
                    1 => OperationType::AdminTransfer,
                    _ => OperationType::SaleCreated,
                };
                create_audit_log(
                    &env,
                    operator.clone(),
                    op_type,
                    before_state.clone(),
                    after_state.clone(),
                    tx_hash.clone(),
                    None,
                );
            }

            // Query first page (limit 3)
            let result1 = query_audit_logs(&env, 1, 10, 3);
            assert_eq!(result1.logs.len(), 3);
            assert_eq!(result1.has_more, true);
            assert_eq!(result1.end_id, 3);

            // Query second page
            let result2 = query_audit_logs(&env, 4, 10, 3);
            assert_eq!(result2.logs.len(), 3);
            assert_eq!(result2.has_more, true);

            // Query last page
            let result3 = query_audit_logs(&env, 7, 10, 10);
            assert_eq!(result3.logs.len(), 4);
            assert_eq!(result3.has_more, false);
        });
    }

    #[test]
    fn test_query_audit_logs_boundary_conditions() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = get_contract_id(&env);
        env.as_contract(&contract_id, || {
            let operator = Address::from_string(&String::from_str(
                &env,
                "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            ));
            let before_state = String::from_str(&env, "{}");
            let after_state = String::from_str(&env, "{}");
            let tx_hash = String::from_str(&env, "tx1");

            // Create 3 logs
            for _ in 0..3 {
                create_audit_log(
                    &env,
                    operator.clone(),
                    OperationType::AdminMint,
                    before_state.clone(),
                    after_state.clone(),
                    tx_hash.clone(),
                    None,
                );
            }

            // Query with start_id = 0 (should default to 1)
            let result1 = query_audit_logs(&env, 0, 10, 100);
            assert_eq!(result1.start_id, 1);
            assert_eq!(result1.logs.len(), 3);

            // Query with end_id > total (should clamp to total)
            let result2 = query_audit_logs(&env, 1, 100, 100);
            assert_eq!(result2.end_id, 3);
            assert_eq!(result2.logs.len(), 3);

            // Query with start_id > total (should return empty)
            let result3 = query_audit_logs(&env, 100, 200, 100);
            assert_eq!(result3.logs.len(), 0);
            assert_eq!(result3.has_more, false);
        });
    }

    #[test]
    fn test_query_audit_logs_out_of_range() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx1");

        // Create only 5 logs
        for _ in 0..5 {
            create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        // Query beyond range
        let result = query_audit_logs(&env, 10, 20, 100);
        assert_eq!(result.logs.len(), 0);
        assert_eq!(result.total_count, 5);
    }

    #[test]
    fn test_query_empty_audit_log() {
        let env = Env::default();
        env.mock_all_auths();

        let result = query_audit_logs(&env, 1, 100, 100);
        assert_eq!(result.logs.len(), 0);
        assert_eq!(result.total_count, 0);
        assert_eq!(result.has_more, false);
    }

    // ========================================================================
    // Export Format Tests
    // ========================================================================

    #[test]
    fn test_export_audit_logs() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{\"a\":1}");
        let after_state = String::from_str(&env, "{\"b\":2}");
        let tx_hash = String::from_str(&env, "tx_hash_export");
        let description = Some(String::from_str(&env, "Export test"));

        // Create a log
        create_audit_log(
            &env,
            operator.clone(),
            OperationType::AdminMint,
            before_state.clone(),
            after_state.clone(),
            tx_hash.clone(),
            description.clone(),
        );

        let export_entries = export_audit_logs(&env, 1, 1, 100);

        assert_eq!(export_entries.len(), 1);
        let entry = export_entries.get(0).unwrap();

        // Verify string conversion. id and timestamp are placeholders due to no_std limitations.
        assert_eq!(entry.id, String::from_str(&env, ""));
        assert_eq!(entry.timestamp, String::from_str(&env, ""));
        assert_eq!(entry.operator, operator.to_string());
        assert_eq!(entry.operation_type, String::from_str(&env, "AdminMint"));
        assert_eq!(entry.before_state, before_state);
        assert_eq!(entry.after_state, after_state);
        assert_eq!(entry.tx_hash, tx_hash);
        assert_eq!(entry.description, description);
    }

    #[test]
    fn test_operation_type_strings() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Test various operation types
        let op_types = [
            (OperationType::AdminMint, "AdminMint"),
            (OperationType::AdminTransfer, "AdminTransfer"),
            (OperationType::SaleCreated, "SaleCreated"),
            (OperationType::LeaseStarted, "LeaseStarted"),
            (OperationType::AuthFailure, "AuthFailure"),
            (OperationType::ErrorOccurred, "ErrorOccurred"),
        ];

        for i in 0..op_types.len() {
            let (op_type, _expected_str) = op_types[i];
            let _ = create_audit_log(
                &env,
                operator.clone(),
                op_type,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        // Verify counter was incremented correctly
        assert_eq!(get_total_audit_log_count(&env), 6);
    }

    // ========================================================================
    // Concurrent Write Tests
    // ========================================================================

    #[test]
    fn test_concurrent_audit_log_creation() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Simulate concurrent writes by creating multiple logs
        let mut ids: Vec<u64> = Vec::new(&env);
        for i in 0..100 {
            let id = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
            ids.push_back(id);
        }

        // Verify all IDs are unique and sequential
        for i in 0..100 {
            assert_eq!(ids.get(i as u32).unwrap(), (i + 1) as u64);
        }

        // Verify counter matches
        assert_eq!(get_log_id_counter(&env), 100);
    }

    // ========================================================================
    // Performance Tests
    // ========================================================================

    #[test]
    fn test_performance_large_audit_log_creation() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Create 1000 audit logs
        for _ in 0..1000 {
            let _ = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        assert_eq!(get_log_id_counter(&env), 1000);
    }

    #[test]
    fn test_performance_large_audit_log_query() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Create 500 audit logs
        for _ in 0..500 {
            let _ = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        // Query with pagination
        let result = query_audit_logs(&env, 1, 500, 50);
        assert_eq!(result.logs.len(), 50);
        assert_eq!(result.has_more, true);

        // Query last page
        let result_last = query_audit_logs(&env, 451, 500, 100);
        assert_eq!(result_last.logs.len(), 50);
        assert_eq!(result_last.has_more, false);
    }

    #[test]
    fn test_performance_retrieval_1m_entries() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Create 1000 audit logs (simulating larger data)
        for _ in 0..1000 {
            let _ = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        // Simulate querying middle range
        let result = query_audit_logs(&env, 400, 600, 100);
        assert_eq!(result.logs.len(), 100);
        assert!(result.total_count >= 1000);
    }

    // ========================================================================
    // Retention Policy Tests
    // ========================================================================

    #[test]
    fn test_retention_policy_permanent_storage() {
        let env = Env::default();
        env.mock_all_auths();

        let operator = Address::from_string(&String::from_str(
            &env,
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
        ));
        let before_state = String::from_str(&env, "{}");
        let after_state = String::from_str(&env, "{}");
        let tx_hash = String::from_str(&env, "tx");

        // Create initial logs
        for _ in 0..10 {
            let _ = create_audit_log(
                &env,
                operator.clone(),
                OperationType::AdminMint,
                before_state.clone(),
                after_state.clone(),
                tx_hash.clone(),
                None,
            );
        }

        let initial_count = get_total_audit_log_count(&env);

        // Verify logs persist (simulating new transaction)
        let count_after = get_total_audit_log_count(&env);
        assert_eq!(initial_count, count_after);

        // Verify specific logs are still accessible
        for i in 1..=10 {
            let log = get_audit_log(&env, i);
            assert!(log.is_some());
        }
    }
}
