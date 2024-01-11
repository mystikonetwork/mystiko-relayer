use crate::common::{create_default_database_in_memory, create_default_server_config};
use mystiko_relayer::configs::account::AccountConfig;
use mystiko_relayer::handler::account::handler::Account;
use mystiko_relayer::handler::account::AccountHandler;
use std::sync::Arc;

#[actix_rt::test]
async fn test_find_by_chain_id_success() {
    // create db
    let db = create_default_database_in_memory().await;
    // create server config
    let server_config = create_default_server_config(true).await;
    let result = Account::new(
        Arc::new(db),
        server_config
            .accounts
            .values()
            .cloned()
            .collect::<Vec<AccountConfig>>()
            .as_slice(),
    )
    .await;
    assert!(result.is_ok());
    let handler = result.unwrap();
    // find by chain id
    let result = handler.find_by_chain_id(5).await;
    assert!(result.is_ok());
    let account = result.unwrap();
    assert_eq!(account.len(), 1usize);
    assert_eq!(account[0].data.chain_id, 5);
    assert_eq!(
        account[0].data.chain_address,
        "0x4d870a75d6552a0199610a460a65116b552de0d9"
    );
    assert!(account[0].data.available);
    assert_eq!(account[0].data.supported_erc20_tokens, ["mtt"]);
    assert_eq!(account[0].data.balance_alarm_threshold, 0.05);
    assert_eq!(account[0].data.balance_check_interval_ms, 500000);
    assert!(!account[0].data.insufficient_balances);
}
