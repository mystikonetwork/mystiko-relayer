pub mod handler;

use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait AccountHandler<A>: Debug + Send + Sync {
    type Error;

    async fn find_by_chain_id(&self, chain_id: u64) -> Result<Vec<A>, Self::Error>;
}
