use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use derive_more::Display;
use mystiko_relayer_types::response::{ApiResponse, ResponseCode};
use mystiko_storage::StorageError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RelayerServerError {
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
    #[error(transparent)]
    StorageError(#[from] StorageError),
    #[error(transparent)]
    Secp256k1Error(#[from] secp256k1::Error),
    #[error("transaction queue send got error: {0:?}")]
    QueueSendError(String),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("provider type unspecified")]
    ProviderTypeUnspecifiedError(),
}

#[derive(Debug, Display, derive_more::Error)]
pub enum ResponseError {
    #[display(fmt = "unknown error")]
    Unknown,
    #[display(fmt = "get minimum gas fee failed")]
    GetMinimumGasFeeFailed,
    #[display(fmt = "relayer server database error")]
    DatabaseError,
    #[display(fmt = "account not found in database")]
    AccountNotFoundInDatabase,
    #[display(fmt = "repeated transaction")]
    RepeatedTransaction,
    #[display(fmt = "request data validate error: {}", error)]
    ValidateError { error: String },
    #[display(fmt = "unsupported transaction")]
    UnsupportedTransaction,
    #[display(fmt = "transaction channel error: {}", error)]
    TransactionChannelError { error: String },
    #[display(fmt = "transaction id {} not found", id)]
    TransactionNotFound { id: String },
    #[display(fmt = "get chain id {} gas price error", chain_id)]
    GetGasPriceError { chain_id: u64 },
    #[display(fmt = "chain id {} not found in relayer config", chain_id)]
    ChainIdNotFoundInRelayerConfig { chain_id: u64 },
    #[display(fmt = "execute transaction failed: {}", error)]
    TransactionFailed { error: String },
}

impl actix_web::error::ResponseError for ResponseError {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let response_json: ApiResponse<()> = ApiResponse {
            code: get_error_code(self) as i32,
            data: None,
            message: Some(self.to_string()),
        };
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(response_json)
    }
}

pub fn get_error_code(error: &ResponseError) -> ResponseCode {
    match error {
        ResponseError::Unknown => ResponseCode::Failed,
        ResponseError::GetMinimumGasFeeFailed => ResponseCode::GetMinimumGasFeeFailed,
        ResponseError::DatabaseError => ResponseCode::DatabaseError,
        ResponseError::RepeatedTransaction => ResponseCode::RepeatedTransaction,
        ResponseError::ValidateError { .. } => ResponseCode::ValidateError,
        ResponseError::UnsupportedTransaction => ResponseCode::UnsupportedTransaction,
        ResponseError::TransactionChannelError { .. } => ResponseCode::TransactionChannelError,
        ResponseError::TransactionNotFound { .. } => ResponseCode::TransactionNotFound,
        ResponseError::GetGasPriceError { .. } => ResponseCode::GetGasPriceError,
        ResponseError::ChainIdNotFoundInRelayerConfig { .. } => ResponseCode::ChainIdNotFound,
        ResponseError::AccountNotFoundInDatabase => ResponseCode::AccountNotFoundInDatabase,
        ResponseError::TransactionFailed { .. } => ResponseCode::Failed,
    }
}
