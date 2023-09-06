use log::LevelFilter;
use mystiko_relayer::database::init_sqlite_database;

const SQLITE_DATABASE_PATH: &str = "./tests/files/sqlite_database_test.db";

#[actix_rt::test]
async fn test_init_sqlite_database() {
    // try init logger
    let _ = env_logger::builder()
        .filter_module("mystiko_relayer", LevelFilter::Debug)
        .try_init();
    let sqlite = init_sqlite_database(SQLITE_DATABASE_PATH).await;
    assert!(sqlite.is_ok());
    // delete database file
    std::fs::remove_file(SQLITE_DATABASE_PATH).unwrap();
}
