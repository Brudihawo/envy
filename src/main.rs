#![feature(async_iterator)]
#![feature(async_closure)]
use once_cell::sync::{Lazy, OnceCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{Html, IntoResponse};
use axum::{routing::get, Router};
use cfg_if::cfg_if;
use tokio::io::AsyncReadExt;

use pulldown_cmark::{Options, Parser};

use envy::file::{File, PaperMeta};
use tracing_subscriber;

const NOTES_PATH: &str = "/home/hawo/notes";

#[derive(Default)]
struct Notes {
    papers: HashMap<String, File>,
    daily: HashMap<String, File>,
    other: HashMap<String, File>,
}

fn html_page(title: &str, body_pre: &str, body: &str) -> Html<String> {
    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<meta charset=\"UTF-8\"/>
<link rel=\"stylesheet\" href=\"/style.css\">
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

static NOTES: Lazy<Arc<Mutex<Notes>>> = Lazy::new(|| Arc::new(Mutex::new(Notes::default())));

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/favicon.ico", get(favicon))
        .route("/style.css", get(style))
        .route("/*path.md", get(get_file));

    let address = "localhost:6969";

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}

async fn favicon() -> impl IntoResponse {
    let bytes = include_bytes!("../assets/favicon.ico");
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
    let files = WalkDir::new(NOTES_PATH)
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
            let file = tokio::fs::File::open(e.path()).await.expect("file exists");
            let metadata = file.metadata().await.expect("file has readable metadata");
            File {
                modified: metadata.modified().expect("fstat is available"),
                path: e.path().to_path_buf(),
                loaded_content: None,
                meta: None,
            }
        });

    for file in files {
        let file = file.await;
        let parent = file
            .path
            .parent()
            .expect("must have at least parent of note folder");
        match parent.file_name().unwrap().to_str().unwrap() {
            "papers" => {
                NOTES
                    .lock()
                    .unwrap()
                    .papers
                    .entry(file.path.to_str().unwrap().to_string())
                    .and_modify(|note| {
                        if note.modified < file.modified {
                            *note = file.with_meta();
                        }
                    })
                    .or_insert(file.with_meta());
            }
            "daily" => {
                NOTES
                    .lock()
                    .unwrap()
                    .daily
                    .entry(file.path.to_str().unwrap().to_string())
                    .and_modify(|note| {
                        if note.modified < file.modified {
                            *note = file.clone();
                        }
                    })
                    .or_insert(file);
            }
            _ => {
                NOTES
                    .lock()
                    .unwrap()
                    .other
                    .entry(file.path.to_str().unwrap().to_string())
                    .and_modify(|note| {
                        if note.modified < file.modified {
                            *note = file.clone();
                        }
                    })
                    .or_insert(file);
            }
        }
    }

    let mut papers = String::new();
    papers.push_str("<h2>Paper Notes</h2>\n  <ul id=papers>");
    //         file.write(
    //             f"""<h2>Paper-Notes</h2>
    // <input type="text" id="paper_search" onkeyup="filter('papers', 'paper_search')" placeholder="Search Tags or Names">
    // {filter_script}
    // <div style="height:50vh;width:100%;overflow:scroll;auto;padding-top:10px;">
    // <ul id="papers">
    // """
    for (_path, paper) in NOTES.lock().unwrap().papers.iter() {
        let meta = &paper.meta.as_ref().unwrap();
        // f'<li authors="{authors}" tags="{tags}" title="{title}"><strong>{title}</strong></br>{year}<em>{authors}</em></br><a href="{fpath}">{fname}</a></li>\n'
        papers.push_str(&format!("<li><strong>{title}</strong></br>{year} <em>{authors}</em></br><a href=\"{path}\">{fname}</a></li>", 
            title=meta.bibtex.title, 
            authors=meta.bibtex.author, 
            year=meta.bibtex.year, 
            path=paper.path.strip_prefix(NOTES_PATH).unwrap().display(), 
            fname=paper.path.file_name().unwrap().to_str().unwrap()));
    }
    papers.push_str("</ul>");
    html_page("Envy - Note Viewer", "", &papers)
}

async fn root() -> Html<String> {
    index_page().await
}
