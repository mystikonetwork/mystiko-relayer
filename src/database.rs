use crate::document::account::AccountCollection;
use crate::document::transaction::TransactionCollection;
use anyhow::Result;
use log::info;
use mystiko_storage::{Collection, Document, MigrationHistory, SqlStatementFormatter, StatementFormatter, Storage};
use mystiko_storage_sqlite::SqliteStorage;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct Database<F: StatementFormatter, S: Storage> {
    pub accounts: AccountCollection<F, S>,
    pub transactions: TransactionCollection<F, S>,
}

impl<F: StatementFormatter, S: Storage> Database<F, S> {
    pub fn new(formatter: F, storage: S) -> Self {
        let collection = Arc::new(Collection::new(formatter, storage));
        Database {
            accounts: AccountCollection::new(collection.clone()),
            transactions: TransactionCollection::new(collection),
        }
    }

    pub async fn migrate(&self) -> Result<Vec<Document<MigrationHistory>>> {
        let migrations: Vec<Document<MigrationHistory>> =
            vec![self.accounts.migrate().await?, self.transactions.migrate().await?];
        Ok(migrations)
    }
}

pub async fn init_sqlite_database(path: Option<String>) -> Result<Database<SqlStatementFormatter, SqliteStorage>> {
    let storage = init_sqlite_storage(path).await?;
    let database = Database::new(SqlStatementFormatter::sqlite(), storage);
    database.migrate().await?;
    Ok(database)
}

async fn init_sqlite_storage(path: Option<String>) -> Result<SqliteStorage> {
    if let Some(path) = path {
        if !Path::new(&path).exists() {
            info!("path {} db file not exists, create sqlite db file", &path);
            let mut file = File::create(&path)?;
            file.write_all(b"")?;
        }
        return Ok(SqliteStorage::from_path(path).await?);
    }
    Ok(SqliteStorage::from_memory().await?)
}
