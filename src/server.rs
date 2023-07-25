use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use warp::hyper::body::Bytes;
use warp::Filter;

use crate::db_client::{SqliteDbClient, SqliteDbClientError};

#[derive(Serialize, Deserialize)]
pub struct ImitateRandomUserResponse {
    user_name: String,
    imitation: String,
}

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
                    match err {
                        SqliteDbClientError::UnknownUser(_) => warp::reply::with_status(
                            format!("Unknown user: {}", user_name),
                            warp::http::StatusCode::NOT_FOUND,
                        ),
                        _ => warp::reply::with_status(
                            "Internal error".to_owned(),
                            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    }
                }
            }
        });

    let client4 = client.clone();
    let imitate_random_user = warp::path!("random-user" / "imitation")
        .and(warp::get())
        .and(warp::any().map(move || client4.clone()))
        .then(|client: SqliteDbClient| async move {
            // girl this shit is s ofucked
            let user_name = match client.get_random_user().await {
                Ok(user_name) => user_name,
                Err(err) => {
                    println!("Error getting random username: {}", err);
                    return warp::reply::with_status(
                        "Internal error".to_owned(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            };

            let imitation = match client.imitate_user(&user_name).await {
                Ok(imitation) => imitation,
                Err(err) => {
                    println!(
                        "Failed to imitate user that was \
                        literally just randomly selected: {:?}",
                        err
                    );

                    // If we can't resolve the username we just randomly
                    // selected FROM the database, that's either a problem or
                    // it's just a result of the last get_random_user call and
                    // the imitation call not being in the same transaction,
                    //
                    // a thing about which I do not care presently.
                    return warp::reply::with_status(
                        "Internal error".to_owned(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            };

            let serialized = match serde_json::to_string(&ImitateRandomUserResponse {
                user_name,
                imitation,
            }) {
                Ok(serialized) => serialized,
                Err(err) => {
                    println!("Serialization failed: {}", err);
                    return warp::reply::with_status(
                        "Internal error".to_owned(),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    );
                }
            };

            warp::reply::with_status(serialized, warp::http::StatusCode::OK)
        });

    warp::serve(add_message.or(imitate).or(imitate_random_user))
}

pub async fn serve(client: SqliteDbClient, host: SocketAddr) -> () {
    let server = make_server(client);
    server.run(host).await
}
