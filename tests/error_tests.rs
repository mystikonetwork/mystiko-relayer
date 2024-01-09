use mystiko_relayer::error::{get_error_code, ResponseError};
use mystiko_relayer_types::response::ResponseCode;

#[test]
fn test_get_error_code() {
    let code = get_error_code(&ResponseError::Unknown);
    assert_eq!(code, ResponseCode::Failed);
    let code = get_error_code(&ResponseError::GetMinimumGasFeeFailed);
    assert_eq!(code, ResponseCode::GetMinimumGasFeeFailed);
    let code = get_error_code(&ResponseError::DatabaseError);
    assert_eq!(code, ResponseCode::DatabaseError);
    let code = get_error_code(&ResponseError::RepeatedTransaction);
    assert_eq!(code, ResponseCode::RepeatedTransaction);
    let code = get_error_code(&ResponseError::ValidateError {
        error: "err".to_string(),
    });
    assert_eq!(code, ResponseCode::ValidateError);
    let code = get_error_code(&ResponseError::TransactionChannelError {
        error: "err".to_string(),
    });
    assert_eq!(code, ResponseCode::TransactionChannelError);
    let code = get_error_code(&ResponseError::TransactionNotFound { id: "id".to_string() });
    assert_eq!(code, ResponseCode::TransactionNotFound);
    let code = get_error_code(&ResponseError::GetGasPriceError { chain_id: 5 });
    assert_eq!(code, ResponseCode::GetGasPriceError);
    let code = get_error_code(&ResponseError::ChainIdNotFoundInRelayerConfig { chain_id: 5 });
    assert_eq!(code, ResponseCode::ChainIdNotFound);
    let code = get_error_code(&ResponseError::AccountNotFoundInDatabase);
    assert_eq!(code, ResponseCode::AccountNotFoundInDatabase);
    let code = get_error_code(&ResponseError::TransactionFailed {
        error: "err".to_string(),
    });
    assert_eq!(code, ResponseCode::Failed);
}
