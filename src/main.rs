#![feature(async_iterator)]
#![feature(async_closure)]

use axum::extract::State;
use axum::{routing::get, Router};

use envy::file_requests::{favicon, style, Envy};
use tracing_subscriber;

type ServerState = State<Envy>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let envy = Envy::build_database("/home/hawo/notes").await;

    let app = Router::new()
        .route(
            "/",
            get(async move |nvy: ServerState| nvy.index_page().await).with_state(envy.clone()),
        )
        .route("/style.css", get(style))
        .route("/favicon.ico", get(favicon))
        .route(
            "/*path",
            get(async move |nvy: ServerState, uri| nvy.get_file(uri).await)
                .with_state(envy.clone()),
        );
    // TODO: handle links with tags

    let address = "localhost:6969";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}
