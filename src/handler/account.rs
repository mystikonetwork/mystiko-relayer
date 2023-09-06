use crate::configs::AccountConfig;
use crate::database::Database;
use crate::document::account::{Account, AccountColumn};
use crate::error::RelayerServerError;
use crate::types::Result;
use log::debug;
use mystiko_protos::storage::v1::SubFilter;
use mystiko_storage::{Document, StatementFormatter, Storage};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};
use std::str::FromStr;
use std::sync::Arc;

pub struct AccountHandler<F: StatementFormatter, S: Storage> {
    db: Arc<Database<F, S>>,
}

impl<F, S> AccountHandler<F, S>
where
    F: StatementFormatter,
    S: Storage,
{
    pub async fn new(db: Arc<Database<F, S>>, accounts: &[AccountConfig]) -> Result<Self> {
        init_data(&db, accounts).await?;
        Ok(Self { db })
    }

    pub async fn find_by_chain_id(&self, chain_id: u64) -> Result<Vec<Document<Account>>> {
        let query_filter = SubFilter::equal(AccountColumn::ChainId, chain_id);
        self.db
            .accounts
            .find(query_filter)
            .await
            .map_err(RelayerServerError::StorageError)
    }
}

async fn init_data<F, S>(db: &Arc<Database<F, S>>, accounts: &[AccountConfig]) -> Result<()>
where
    F: StatementFormatter,
    S: Storage,
{
    debug!("init accounts database");

    // clear data
    db.accounts
        .delete_all()
        .await
        .map_err(RelayerServerError::StorageError)?;

    // batch insert accounts data
    let mut docs = Vec::new();
    for account in accounts.iter() {
        // private key to public key
        let address = get_address(&account.private_key)?;
        let doc = Account {
            chain_address: address,
            chain_id: account.chain_id,
            available: account.available,
            supported_erc20_tokens: account
                .supported_erc20_tokens
                .iter()
                .map(|token| token.to_lowercase())
                .collect(),
            balance_alarm_threshold: account.balance_alarm_threshold,
            balance_check_interval_ms: account.balance_check_interval_ms,
            insufficient_balances: true,
        };
        docs.push(doc);
    }
    db.accounts
        .insert_batch(&docs)
        .await
        .map_err(RelayerServerError::StorageError)?;

    Ok(())
}

fn get_address(secret_key: &str) -> Result<String> {
    let secp = Secp256k1::new();
    let sk = SecretKey::from_str(secret_key).map_err(RelayerServerError::Secp256k1Error)?;
    let pk = PublicKey::from_secret_key(&secp, &sk);
    let bytes = pk.serialize_uncompressed();
    let hash = Keccak256::digest(&bytes[1..]);
    let address = format!("0x{}", hex::encode(&hash[12..]));
    Ok(address)
}
