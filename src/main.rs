#![feature(async_iterator)]
#![feature(async_closure)]
use axum::http::{header, HeaderMap};
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use cfg_if::cfg_if;
use tokio::io::AsyncReadExt;

use tracing_subscriber;
use envy::file_requests::{get_file, root, favicon};


cfg_if! {
    if #[cfg(reload_css)] {
        async fn style() -> impl IntoResponse {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());

            (headers, include_str!("style.css"))
        }
    } else {
        async fn style() -> impl IntoResponse {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());

            let mut file = tokio::fs::File::open("src/style.css").await.unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).await.unwrap();
            (headers, content)
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/favicon.ico", get(favicon))
        .route("/style.css", get(style))
        // .route("/api/query_all", get(query_all))
        .route("/*path", get(get_file)); // TODO: handle links with tags

    let address = "localhost:6969";

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}

