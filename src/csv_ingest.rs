use std::{error::Error, path::PathBuf};

use tokio::fs::File;
use tokio_stream::StreamExt;

use crate::db_client::SqliteDbClient;

pub async fn ingest_csv(client: SqliteDbClient, input_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut reader = csv_async::AsyncReaderBuilder::new()
        .has_headers(false)
        .create_reader(File::open(input_path).await?);
    let mut records: csv_async::StringRecordsStream<'_, File> = reader.records();
    let mut n_processed: u64 = 0;
    let mut last_checkpoint: u64 = 0;

    while let Some(record) = records.next().await {
        let record: csv_async::StringRecord = record?;
        let (Some(user_name), Some(message)) = (record.get(0), record.get(1)) else {
            return Err(Box::<dyn Error>::from("Rows need at least 2 cells"))
        };
        client.add_message(user_name, message).await?;
        n_processed += 1;
        if n_processed - last_checkpoint >= 100 {
            last_checkpoint = n_processed - (n_processed % 100);
            println!("Processed {} messages", last_checkpoint);
        }
    }

    Ok(())
}
