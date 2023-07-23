mod db_client;

use warp::Filter;
use warp::hyper::body::Bytes;

use db_client::SqliteDbClient;

#[tokio::main]
async fn main() {
    let client = SqliteDbClient::new("sqlite://messages.db?mode=rwc")
        .await
        .expect("Failed to create db client");

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
        .then(|user_id, client| async move { format!("Imitation of user {} requested", user_id) });

    let all = add_message.or(imitate);

    println!("Starting server...");
    let socket_addr: std::net::SocketAddr = "127.0.0.1:3030"
        .parse()
        .expect("Invalid socket address format");
    warp::serve(all).run(socket_addr).await;
}
