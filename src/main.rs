#![feature(async_iterator)]
use std::path::PathBuf;

use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{Html, IntoResponse};
use axum::{routing::get, Router};
use pulldown_cmark::{Options, Parser};
use tokio::io::AsyncReadExt;
use tracing_subscriber;

use envy::bibtex::BibtexEntry;

const NOTES_PATH: &str = "/home/hawo/notes";

struct InterimPaperMeta {
    tags: Option<Vec<String>>,
    bibtex: String,
    pdf: PathBuf,
}

enum FType {
    PaperNote {
        content: String,
        meta: Option<String>,
    },
    OtherNote {
        content: String,
    },
    Pdf {
        bytes: Vec<u8>,
    },
}

struct File {
    modified: std::time::SystemTime,
    path: std::path::PathBuf,
    content: String,
}

fn html_page(title: &str, body_pre: &str, body: &str) -> Html<String> {
    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<meta charset=\"UTF-8\"/>
<title>{title}</title>
<body>
    {body_pre}
    {body}
</body>
</html>"
    )
    .into()
}

async fn get_file(path: Uri) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let file = match tokio::fs::File::open(&str_path).await {
        Ok(file) => file,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                html_page(
                    "Error 404, File not Found",
                    "",
                    &format!("Could not find file {str_path}"),
                ),
            )
                .into_response();
        }
    };

    (headers, compile_markdown(file).await).into_response()
}

async fn compile_markdown(mut file: tokio::fs::File) -> Html<String> {
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).await.unwrap();
    file_contents = file_contents.replace(NOTES_PATH, "");

    let parser = Parser::new_ext(&file_contents, Options::all());
    let mut html_output = String::new();

    pulldown_cmark::html::push_html(&mut html_output, parser);

    html_page(
        "Hello, World",
        "<img width=\"32\" height=\"32\" src=\"/favicon.ico\">",
        &html_output,
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/favicon.ico", get(favicon))
        .route("/*path.md", get(get_file));

    let address = "localhost:6969";

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}

async fn favicon() -> impl IntoResponse {
    let mut bytes = Vec::new();
    let mut file = tokio::fs::File::open("/home/hawo/workspace/envy/assets/favicon.ico")
        .await
        .unwrap();
    file.read_to_end(&mut bytes).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"favicon.ico\"".parse().unwrap(),
    );

    (headers, bytes)
}

async fn index_page() -> Html<String> {
    use walkdir::WalkDir;
    let infos = WalkDir::new(NOTES_PATH)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let m = e.metadata().expect("entry metadata is readable");
            let ext = e.path().extension();
            if ext.is_some() {
                m.is_file() && ext.unwrap() == "md"
            } else {
                false
            }
        })
        .map(|e| async move {
            // TODO handle pdfs
            let mut contents = String::new();
            tokio::fs::File::open(e.path())
                .await
                .expect("file exists")
                .read_to_string(&mut contents)
                .await
                .unwrap();
            if contents.starts_with("---") {
                // has yaml frontmatter
                // try to find end of frontmatter
                let mut parts = contents.split("---\n");
                let _empty = parts.next().expect("metadata is present");
                // TODO: handle empty metadata
                let meta = parts.next().expect("metadata is present");
                // TODO: handle empty content
                let _file_content = parts.next().expect("there is some content in the file");
                meta.to_string()
            } else {
                e.path().display().to_string()
            }
        });
    let mut s = String::new();
    for info in infos {
        let info = info.await;
        s.push_str(&format!("<p>{info}</p>"))
    }
    html_page("Envy - Note Viewer", "", &s)
}

async fn root() -> Html<String> {
    index_page().await
}
