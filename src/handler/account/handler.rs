use crate::configs::account::AccountConfig;
use crate::database::account::{Account as DocumentAccount, AccountColumn};
use crate::database::Database;
use crate::error::RelayerServerError;
use crate::handler::account::AccountHandler;
use crate::handler::types::Result;
use async_trait::async_trait;
use log::debug;
use mystiko_protos::storage::v1::SubFilter;
use mystiko_storage::{Document, StatementFormatter, Storage};
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug)]
pub struct Account<F: StatementFormatter, S: Storage> {
    db: Arc<Database<F, S>>,
}

#[async_trait]
impl<F, S> AccountHandler<Document<DocumentAccount>> for Account<F, S>
where
    F: StatementFormatter + 'static,
    S: Storage + 'static,
{
    type Error = RelayerServerError;

    async fn find_by_chain_id(&self, chain_id: u64) -> Result<Vec<Document<DocumentAccount>>> {
        let query_filter = SubFilter::equal(AccountColumn::ChainId, chain_id);
        self.db
            .accounts
            .find(query_filter)
            .await
            .map_err(RelayerServerError::StorageError)
    }
}

impl<F, S> Account<F, S>
where
    F: StatementFormatter,
    S: Storage,
{
    pub async fn new(db: Arc<Database<F, S>>, accounts: &[AccountConfig]) -> Result<Self> {
        let account = Account { db };
        account.init_data(accounts).await?;
        Ok(account)
    }

    async fn init_data(&self, accounts: &[AccountConfig]) -> Result<()> {
        debug!("init accounts database");
        // clear data
        self.db
            .accounts
            .delete_all()
            .await
            .map_err(RelayerServerError::StorageError)?;
        // batch insert accounts data
        let mut docs = Vec::new();
        for account in accounts.iter() {
            // private key to public key
            let address = self.get_address(&account.private_key)?;
            let supported_erc20_tokens: Vec<String> = account.supported_erc20_tokens.values().cloned().collect();
            let doc = DocumentAccount {
                chain_address: address,
                chain_id: account.chain_id,
                available: account.available,
                supported_erc20_tokens: supported_erc20_tokens
                    .iter()
                    .map(|token| token.to_lowercase())
                    .collect(),
                balance_alarm_threshold: account.balance_alarm_threshold,
                balance_check_interval_ms: account.balance_check_interval_ms,
                insufficient_balances: false,
            };
            docs.push(doc);
        }
        self.db
            .accounts
            .insert_batch(&docs)
            .await
            .map_err(RelayerServerError::StorageError)?;
        Ok(())
    }

    fn get_address(&self, secret_key: &str) -> Result<String> {
        let secp256k1 = Secp256k1::new();
        let sk = SecretKey::from_str(secret_key).map_err(RelayerServerError::Secp256k1Error)?;
        let pk = PublicKey::from_secret_key(&secp256k1, &sk);
        let bytes = pk.serialize_uncompressed();
        let hash = Keccak256::digest(&bytes[1..]);
        let address = format!("0x{}", hex::encode(&hash[12..]));
        Ok(address)
    }
}
