use crate::database::Database;
use crate::document::transaction::{Transaction, TransactionColumn};
use crate::error::RelayerServerError;
use crate::types::Result;
use mystiko_protos::storage::v1::SubFilter;
use mystiko_relayer_types::{TransactRequestData, TransactStatus};
use mystiko_storage::{Document, StatementFormatter, Storage};
use mystiko_utils::convert::u256_to_biguint;
use std::sync::Arc;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Debug, Clone)]
pub struct UpdateTransactionOptions {
    #[builder(default, setter(strip_option))]
    pub status: Option<TransactStatus>,
    #[builder(default, setter(strip_option))]
    pub error_message: Option<String>,
    #[builder(default, setter(strip_option))]
    pub transaction_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransactionHandler<F: StatementFormatter, S: Storage> {
    db: Arc<Database<F, S>>,
}

impl<F, S> TransactionHandler<F, S>
where
    F: StatementFormatter,
    S: Storage,
{
    pub fn new(db: Arc<Database<F, S>>) -> Self {
        Self { db }
    }

    pub async fn create_by_request(&self, data: TransactRequestData) -> Result<Document<Transaction>> {
        let transaction = Transaction {
            chain_id: data.chain_id,
            transaction_type: data.transaction_type,
            bridge_type: data.bridge_type,
            status: TransactStatus::Queued,
            pool_address: data.pool_address,
            asset_symbol: data.asset_symbol,
            asset_decimals: data.asset_decimals,
            circuit_type: data.circuit_type,
            proof: serde_json::to_string(&data.contract_param.proof)?,
            root_hash: u256_to_biguint(&data.contract_param.root_hash),
            output_commitments: if data.contract_param.out_commitments.is_empty() {
                None
            } else {
                Some(
                    data.contract_param
                        .out_commitments
                        .iter()
                        .map(u256_to_biguint)
                        .collect(),
                )
            },
            signature: data.signature,
            serial_numbers: if data.contract_param.serial_numbers.is_empty() {
                None
            } else {
                Some(data.contract_param.serial_numbers.iter().map(u256_to_biguint).collect())
            },
            sig_hashes: if data.contract_param.sig_hashes.is_empty() {
                None
            } else {
                Some(data.contract_param.sig_hashes.iter().map(u256_to_biguint).collect())
            },
            sig_pk: serde_json::to_string(&data.contract_param.sig_pk).unwrap(),
            public_amount: u256_to_biguint(&data.contract_param.public_amount),
            gas_relayer_fee_amount: u256_to_biguint(&data.contract_param.relayer_fee_amount),
            out_rollup_fees: if data.contract_param.out_rollup_fees.is_empty() {
                None
            } else {
                Some(
                    data.contract_param
                        .out_rollup_fees
                        .iter()
                        .map(u256_to_biguint)
                        .collect(),
                )
            },
            public_recipient: data.contract_param.public_recipient.to_string(),
            relayer_recipient_address: data.contract_param.relayer_address.to_string(),
            out_encrypted_notes: if data.contract_param.out_encrypted_notes.is_empty() {
                None
            } else {
                Some(
                    data.contract_param
                        .out_encrypted_notes
                        .iter()
                        .map(|note| note.to_string())
                        .collect::<Vec<String>>(),
                )
            },
            random_auditing_public_key: u256_to_biguint(&data.contract_param.random_auditing_public_key),
            error_message: None,
            transaction_hash: None,
        };
        self.db
            .transactions
            .insert(&transaction)
            .await
            .map_err(RelayerServerError::StorageError)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Document<Transaction>>> {
        self.db
            .transactions
            .find_by_id(id)
            .await
            .map_err(RelayerServerError::StorageError)
    }

    pub async fn update_by_id(
        &self,
        id: &str,
        options: &UpdateTransactionOptions,
    ) -> Result<Option<Document<Transaction>>> {
        self.update(self.find_by_id(id).await?, options).await
    }

    pub async fn is_repeated_transaction(&self, signature: &str) -> Result<bool> {
        let query_filter = SubFilter::equal(TransactionColumn::Signature, signature);
        let transactions = self.db.transactions.find(query_filter).await?;
        for transaction in transactions {
            if transaction.data.status != TransactStatus::Failed {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn update(
        &self,
        existing_transaction: Option<Document<Transaction>>,
        options: &UpdateTransactionOptions,
    ) -> Result<Option<Document<Transaction>>> {
        if let Some(mut existing_transaction) = existing_transaction {
            let mut has_update = false;
            if let Some(status) = &options.status {
                if status.ne(&existing_transaction.data.status) {
                    existing_transaction.data.status = status.clone();
                    has_update = true;
                }
            }
            if let Some(error_message) = &options.error_message {
                let update_error_message = existing_transaction
                    .data
                    .error_message
                    .as_ref()
                    .map(|existing_error| error_message.ne(existing_error))
                    .unwrap_or(true);
                if update_error_message {
                    existing_transaction.data.error_message = Some(error_message.to_string());
                    has_update = true;
                }
            }
            if let Some(transaction_hash) = &options.transaction_hash {
                let update_transaction_hash = existing_transaction
                    .data
                    .transaction_hash
                    .as_ref()
                    .map(|existing_hash| transaction_hash.ne(existing_hash))
                    .unwrap_or(true);
                if update_transaction_hash {
                    existing_transaction.data.transaction_hash = Some(transaction_hash.to_string());
                    has_update = true;
                }
            }
            return if has_update {
                Ok(Some(self.db.transactions.update(&existing_transaction).await?))
            } else {
                Ok(Some(existing_transaction))
            };
        }
        Ok(None)
    }
}
