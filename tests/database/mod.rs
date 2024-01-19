use log::LevelFilter;
use mystiko_relayer::database::init_sqlite_database;
use std::path::Path;
use tempfile::tempdir;

#[actix_rt::test]
async fn test_init_sqlite_database() {
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join(Path::new("test.db"));
    // try init logger
    let _ = env_logger::builder()
        .filter_module("mystiko_relayer", LevelFilter::Debug)
        .try_init();
    let sqlite = init_sqlite_database(Some(db_path.to_string_lossy().to_string())).await;
    assert!(sqlite.is_ok());
    // delete database file
    std::fs::remove_dir_all(db_dir).unwrap();
}
