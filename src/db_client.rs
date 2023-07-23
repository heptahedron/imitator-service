use sqlx::Executor;

#[derive(Clone)]
pub struct SqliteDbClient {
    pool: sqlx::SqlitePool,
}

#[derive(Debug)]
pub enum SqliteDbClientError {
    SqlxError(sqlx::Error),
    Other(String),
}

impl From<sqlx::Error> for SqliteDbClientError {
    fn from(value: sqlx::Error) -> Self {
        SqliteDbClientError::SqlxError(value)
    }
}

pub type SqliteDbClientResult<T> = Result<T, SqliteDbClientError>;

impl SqliteDbClient {
    pub async fn new(url: &str) -> SqliteDbClientResult<Self> {
        let pool = sqlx::SqlitePool::connect(url).await?;
        let client = SqliteDbClient { pool };

        client.init_tables().await?;

        Ok(client)
    }

    async fn init_tables(&self) -> SqliteDbClientResult<()> {
        self.pool
            .execute(
                "CREATE TABLE IF NOT EXISTS user_names ( \
            user_name TEXT PRIMARY KEY, \
            user_id INT NOT NULL \
        )",
            )
            .await?;
        self.pool
            .execute(
                "CREATE TABLE IF NOT EXISTS user_messages ( \
            id INT PRIMARY KEY, \
            digest BLOB UNIQUE, \
            user_id INT NOT NULL, \
            message TEXT NOT NULL
        )",
            )
            .await?;
        // self.pool.execute("CREATE TABLE IF NOT EXISTS sequential_words ( \
        //
        // )");

        Ok(())
    }

    pub async fn add_message(&self, user_name: &str, message: &str) -> SqliteDbClientResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO user_names (user_name, user_id) \
                VALUES (?, abs(random() & ((1 << 31) - 1)))",
        )
        .bind(user_name)
        .execute(&self.pool)
        .await?;

        let user_id =
            sqlx::query_scalar::<_, u32>("SELECT user_id FROM user_names WHERE user_name = ?")
                .bind(user_name)
                .fetch_one(&self.pool)
                .await?;

        let digest_bytes: [u8; 16] = md5::compute(message).into();

        sqlx::query(
            "INSERT OR IGNORE INTO user_messages (digest, user_id, message) \
            VALUES (?, ?, ?)",
        )
        .bind(&digest_bytes[..])
        .bind(user_id)
        .bind(message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
