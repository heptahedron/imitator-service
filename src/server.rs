use std::net::SocketAddr;

use warp::hyper::body::Bytes;
use warp::Filter;

use crate::db_client::SqliteDbClient;

pub fn make_server(
    client: SqliteDbClient,
) -> warp::Server<impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone> {
    let client2 = client.clone();
    let add_message = warp::path!("users" / String / "messages")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(warp::any().map(move || client2.clone()))
        .then(
            |user_name: String, request_body: Bytes, client: SqliteDbClient| async move {
                let Ok(user_name): Result<String, _> = urlencoding::decode(&user_name).map(Into::into) else {
                    return warp::reply::with_status(
                        "Invalid utf8 in username",
                        warp::http::StatusCode::BAD_REQUEST,
                    )
                };

                let message = match std::str::from_utf8(&request_body) {
                    Ok(message) => message,
                    Err(_) => {
                        return warp::reply::with_status(
                            "Invalid utf8 in message",
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
            let Ok(user_name): Result<String, _> = urlencoding::decode(&user_name).map(Into::into) else {
                return warp::reply::with_status(
                    "Invalid utf8 in username".to_owned(),
                    warp::http::StatusCode::BAD_REQUEST,
                )
            };

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

    warp::serve(add_message.or(imitate))
}

pub async fn serve(client: SqliteDbClient, host: SocketAddr) -> () {
    let server = make_server(client);
    server.run(host).await
}
