use crate::error::RelayerServerError;

pub type Result<T> = anyhow::Result<T, RelayerServerError>;
