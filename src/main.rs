#![feature(async_iterator)]
#![feature(async_closure)]

use axum::{routing::get, Router};
use std::path::Path;

use envy::api::query_any;
use envy::file_requests::{favicon, script, style};
use envy::state::{Envy, ServerState};
use envy::watch::watch;
use notify::{recommended_watcher, RecursiveMode, Watcher};

use tracing_subscriber;

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
        .route(
            "/",
            get(async move |nvy: ServerState| nvy.render_index_page()).with_state(envy.clone()),
        )
        .route("/script.js", get(script))
        .route("/style.css", get(style))
        .route("/favicon.ico", get(favicon))
        .route("/api", get(query_any).with_state(envy.clone()))
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
