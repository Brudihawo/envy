use axum::http::Uri;
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use std::path::Path;

use envy::api::{query_meta, query_fulltext};
use envy::file_requests::{favicon, script, style};
use envy::state::{Envy, ServerState};
use envy::watch::watch;
use notify::{recommended_watcher, RecursiveMode, Watcher};

use tracing_subscriber;

async fn index(nvy: ServerState) -> impl IntoResponse {
    nvy.render_index_page()
}

async fn file(nvy: ServerState, uri: Uri) -> impl IntoResponse {
    nvy.get_file(uri).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let envy = Envy::build_database("/home/hawo/notes").await;

    let nvy_watch = envy.clone();
    let mut watcher = recommended_watcher(move |res| watch(res, nvy_watch.clone()))
        .expect("could not create fs watcher");

    watcher
        .watch(Path::new("/home/hawo/notes"), RecursiveMode::Recursive)
        .unwrap();

    let app = Router::new()
        .route("/", get(index).with_state(envy.clone()))
        .route("/script.js", get(script))
        .route("/style.css", get(style))
        .route("/favicon.ico", get(favicon))
        .route("/api/meta", get(query_meta).with_state(envy.clone()))
        .route("/api/fulltext", get(query_fulltext).with_state(envy.clone()))
        .route("/*path", get(file).with_state(envy.clone()));
    // TODO: handle links with tags

    let address = "localhost:6969";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}