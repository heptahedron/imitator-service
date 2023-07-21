use serde::Deserialize;
use warp::Filter;

#[derive(Deserialize)]
pub struct AddMessageRequestBody {
    message: String,
}

#[tokio::main]
async fn main() {
    let add_message = warp::path!("users" / String / "messages")
        .and(warp::post())
        .and(warp::body::json())
        .then(|user_id, request_body: AddMessageRequestBody| async move {
            format!(
                "Added message \"{}\" from user \"{}\"",
                user_id, request_body.message
            )
        });

    println!("Starting server...");
    let socket_addr: std::net::SocketAddr = "127.0.0.1:3030"
        .parse()
        .expect("Invalid socket address format");
    warp::serve(add_message).run(socket_addr).await;
}
