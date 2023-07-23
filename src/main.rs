mod csv_ingest;
mod db_client;
mod server;

use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::StructOpt;

use crate::csv_ingest::ingest_csv;
use crate::db_client::SqliteDbClient;
use crate::server::serve;

#[derive(Debug, StructOpt)]
pub struct ImitatorOptions {
    #[structopt(
        short = "d",
        long = "db",
        default_value = "sqlite://messages.db?mode=rwc"
    )]
    db: String,
    #[structopt(subcommand)]
    subcommand: ImitatorSubcommands,
}

#[derive(Debug, StructOpt)]
pub enum ImitatorSubcommands {
    Serve {
        host: SocketAddr,
    },
    Imitate {
        user_name: String,
    },
    IngestCsv {
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
}

async fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    use ImitatorSubcommands::*;

    let ImitatorOptions { db, subcommand } = ImitatorOptions::from_args();

    let client = SqliteDbClient::new(&db)
        .await
        .expect("Failed to create db client");

    match subcommand {
        Serve { host } => serve(client, host).await,
        Imitate { user_name } => {
            println!("{}", client.imitate_user(&user_name).await?);
        }
        IngestCsv { input } => ingest_csv(client, input).await?,
    };

    Ok(())
}

#[tokio::main]
pub async fn main() {
    real_main().await.unwrap();
}
