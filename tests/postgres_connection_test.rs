use bee::infrastructure::persistence::postgres::PostgresConnection;

#[tokio::test]
async fn test_create_connection() {
    // 跳过如果没有 DATABASE_URL
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let result = PostgresConnection::new(&database_url).await;
    assert!(result.is_ok(), "Should create connection successfully");
}

#[tokio::test]
async fn test_migration_run() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set, skipping test");
            return Ok(());
        }
    };

    let conn = PostgresConnection::new(&database_url).await?;
    conn.migrate().await?;
    Ok(())
}
