use sqlx::Executor;
use thiserror::Error;

#[derive(Clone)]
pub struct SqliteDbClient {
    pool: sqlx::SqlitePool,
    words_regex: regex::Regex,
}

#[derive(Error, Debug)]
pub enum SqliteDbClientError {
    #[error("Unknown user: {0}")]
    UnknownUser(String),
    #[error("Sql error")]
    SqlxError(#[from] sqlx::Error),
    #[error("Other: {0}")]
    Other(String),
}

pub type SqliteDbClientResult<T> = Result<T, SqliteDbClientError>;

impl SqliteDbClient {
    pub async fn new(url: &str) -> SqliteDbClientResult<Self> {
        let pool = sqlx::SqlitePool::connect(url).await?;
        let Ok(words_regex) = regex::Regex::new(r"(\w+|[^\s]+)") else {
            return Err(SqliteDbClientError::Other("Failed to construct regex".to_owned()))
        };
        let client = SqliteDbClient { pool, words_regex };

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
        self.pool
            .execute(
                "CREATE TABLE IF NOT EXISTS sequential_words ( \
            id INT PRIMARY KEY, \
            user_id INT NOT NULL, \
            word_1 TEXT NOT NULL, \
            word_2 TEXT NOT NULL, \
            count INT NOT NULL, \
            CONSTRAINT unique_word_pairs UNIQUE (user_id, word_1, word_2)
        )",
            )
            .await?;

        Ok(())
    }

    pub fn get_message_words<'message, 's: 'message>(
        &'s self,
        message: &'message str,
    ) -> impl Iterator<Item = &'message str> + 'message {
        self.words_regex
            .find_iter(message)
            .map(move |m| m.as_str())
            .filter(|s| !s.is_empty())
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

        let inserted_row_count = sqlx::query(
            "INSERT OR IGNORE INTO user_messages (digest, user_id, message) \
            VALUES (?, ?, ?)",
        )
        .bind(&digest_bytes[..])
        .bind(user_id)
        .bind(message)
        .execute(&self.pool)
        .await?;

        if inserted_row_count.rows_affected() > 0 {
            let word_pairs = [""]
                .into_iter()
                .chain(self.get_message_words(message))
                .chain([""].into_iter())
                .collect::<Vec<_>>()
                .windows(2)
                .map(|pair| match *pair {
                    [word_1, word_2] => (word_1, word_2),
                    _ => panic!("This shouldn't happen"),
                })
                .into_iter()
                .collect::<Vec<_>>();

            for (word_1, word_2) in word_pairs {
                sqlx::query(
                    "INSERT INTO sequential_words (user_id, word_1, word_2, count) \
                    VALUES (?, ?, ?, 1) \
                    ON CONFLICT (user_id, word_1, word_2) \
                    DO UPDATE SET count = count + 1",
                )
                .bind(user_id)
                .bind(word_1)
                .bind(word_2)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn imitate_user(&self, user_name: &str) -> SqliteDbClientResult<String> {
        let user_id =
            sqlx::query_scalar::<_, u32>("SELECT user_id FROM user_names WHERE user_name = ?")
                .bind(user_name)
                .fetch_one(&self.pool)
                .await
                .map_err(|err| match err {
                    sqlx::Error::RowNotFound => {
                        SqliteDbClientError::UnknownUser(user_name.to_owned())
                    }
                    other => other.into(),
                })?;

        let mut current_word = "".to_owned();
        let mut sentence: Vec<String> = vec![];

        for _ in 0..100 {
            let next_words = sqlx::query_as::<_, (String, u32)>(
                "SELECT word_2, count FROM sequential_words \
                WHERE user_id = ? AND word_1 = ? \
                ORDER BY random() LIMIT 10",
            )
            .bind(user_id)
            .bind(&current_word)
            .fetch_all(&self.pool)
            .await?;
            let total_weight: u32 = next_words.iter().map(|(_, count)| *count).sum();
            let mut chosen_index: u32 = rand::random::<u32>() % total_weight;

            for (next_word, count) in next_words {
                if chosen_index < count {
                    current_word = next_word;
                    break;
                } else {
                    chosen_index = chosen_index.checked_sub(count).unwrap_or(0);
                }
            }

            if current_word.len() == 0 {
                break;
            }

            sentence.push(current_word.clone());
        }

        Ok(sentence.join(" "))
    }
}
