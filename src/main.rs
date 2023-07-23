mod csv_ingest;
mod db_client;

use std::path::PathBuf;

use structopt::StructOpt;
use warp::hyper::body::Bytes;
use warp::Filter;

use crate::csv_ingest::ingest_csv;
use crate::db_client::SqliteDbClient;

#[derive(Debug, StructOpt)]
pub struct ImitatorOptions {
    #[structopt(
        short = "d",
        long = "db",
        default_value = "sqlite://messages.db?mode=rwc"
    )]
    db: String,
    #[structopt(subcommand)]
    subcommand: Option<ImitatorSubcommands>,
}

#[derive(Debug, StructOpt)]
pub enum ImitatorSubcommands {
    IngestCsv {
        #[structopt(parse(from_os_str))]
        input: PathBuf,
    },
}

async fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    let ImitatorOptions { db, subcommand } = ImitatorOptions::from_args();

    let client = SqliteDbClient::new(&db)
        .await
        .expect("Failed to create db client");

    match subcommand {
        Some(ImitatorSubcommands::IngestCsv { input }) => {
            ingest_csv(client, input).await?;
            return Ok(());
        }
        None => (),
    };

    let client2 = client.clone();
    let add_message = warp::path!("users" / String / "messages")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(warp::any().map(move || client2.clone()))
        .then(
            |user_name: String, request_body: Bytes, client: SqliteDbClient| async move {
                let message = match std::str::from_utf8(&request_body) {
                    Ok(message) => message,
                    Err(_) => {
                        return warp::reply::with_status(
                            "Invalid utf8",
                            warp::http::StatusCode::BAD_REQUEST,
                        )
                    }
                };

                match client.add_message(&user_name, &message).await {
                    Ok(_) => warp::reply::with_status(
                        "Successfully added message",
                        warp::http::StatusCode::CREATED,
                    ),
                    Err(err) => {
                        println!("Failed to add message: {:?}", err);

                        warp::reply::with_status(
                            "Error",
                            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    }
                }
            },
        );

    let client3 = client.clone();
    let imitate = warp::path!("users" / String / "imitation")
        .and(warp::get())
        .and(warp::any().map(move || client3.clone()))
        .then(|user_name: String, client: SqliteDbClient| async move {
            match client.imitate_user(&user_name).await {
                Ok(sentence) => warp::reply::with_status(sentence, warp::http::StatusCode::OK),
                Err(err) => {
                    println!("Failed to imitate user: {:?}", err);
                    warp::reply::with_status(
                        "Error".to_owned(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            }
        });

    let all = add_message.or(imitate);

    println!("Starting server...");
    let socket_addr: std::net::SocketAddr = "127.0.0.1:3030"
        .parse()
        .expect("Invalid socket address format");
    warp::serve(all).run(socket_addr).await;

    Ok(())
}

#[tokio::main]
pub async fn main() {
    real_main().await.expect("");
}
