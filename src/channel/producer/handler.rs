use crate::channel::producer::ProducerHandler;
use crate::database::transaction::Transaction as DocumentTransaction;
use crate::error::RelayerServerError;
use crate::handler::transaction::{TransactionHandler, UpdateTransactionOptions};
use crate::handler::types::Result;
use async_trait::async_trait;
use log::info;
use mystiko_relayer_types::{TransactRequestData, TransactStatus};
use mystiko_storage::Document;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub struct TransactionProducer<
    T: TransactionHandler<Document<DocumentTransaction>> = Box<
        dyn TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>,
    >,
> {
    sender: Arc<Sender<(String, TransactRequestData)>>,
    transaction_handler: Arc<T>,
}

#[async_trait]
impl<T> ProducerHandler for TransactionProducer<T>
where
    T: TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>,
{
    type Error = RelayerServerError;

    async fn send(&self, data: TransactRequestData) -> Result<Document<DocumentTransaction>> {
        let transaction = self.transaction_handler.create_by_request(data.clone()).await?;
        info!(
            "successfully created a transaction(id = {}, chain_id = {}, spend_type = {:?})",
            &transaction.id, &transaction.data.chain_id, &transaction.data.spend_type
        );

        // send transaction to queue
        let queue = self
            .sender
            .send((transaction.id.clone(), data))
            .await
            .map_err(|e| RelayerServerError::QueueSendError(e.to_string()));

        match queue {
            Ok(_) => {
                info!(
                    "successfully sent a transaction to queue(id = {}, chain_id = {}, spend_type = {:?})",
                    &transaction.id, &transaction.data.chain_id, &transaction.data.spend_type
                );
                Ok(transaction)
            }
            Err(err) => {
                self.transaction_handler
                    .update_by_id(
                        &transaction.id,
                        &UpdateTransactionOptions::builder()
                            .status(TransactStatus::Failed)
                            .error_message(err.to_string())
                            .build(),
                    )
                    .await?;
                Err(err)
            }
        }
    }
}

impl<T> TransactionProducer<T>
where
    T: TransactionHandler<Document<DocumentTransaction>, Error = RelayerServerError>,
{
    pub fn new(
        sender: Arc<Sender<(String, TransactRequestData)>>,
        transaction_handler: Arc<T>,
    ) -> TransactionProducer<T> {
        TransactionProducer {
            sender,
            transaction_handler,
        }
    }
}
