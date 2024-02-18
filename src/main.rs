#![feature(async_iterator)]
#![feature(async_closure)]
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::http::{header, HeaderMap, Response, StatusCode, Uri};
use axum::response::{Html, IntoResponse};
use axum::{routing::get, Router};
use cfg_if::cfg_if;
use tokio::io::AsyncReadExt;

use pulldown_cmark::{Options, Parser, CowStr};

use envy::file::{File, PaperMeta};
use tracing_subscriber;

const NOTES_PATH: &str = "/home/hawo/notes";

#[derive(Default)]
struct Notes {
    papers: HashMap<String, File>,
    daily: HashMap<String, File>,
    other: HashMap<String, File>,
}

fn note_page(title: &str, body_pre: &str, body: &str) -> Html<String> {
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

async fn get_file(path: Uri) -> Response<axum::body::Body> {
    let p = path.path();
    // TODO: query file cache first

    if p.ends_with(".pdf") {
        return get_pdf(path).await;
    } 
    if p.ends_with(".md") {
        return get_md(path).await;
    } 

    todo!("Unhandled file type")
}

async fn get_pdf(path: Uri) -> Response<axum::body::Body> {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let mut file = match tokio::fs::File::open(&str_path).await {
        Ok(file) => file,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                note_page(
                    "Error 404, File not Found",
                    "",
                    &format!("Could not find file {str_path}"),
                ),
            )
                .into_response();
        }
    };
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf).await.map_err(|err| {
            (
                StatusCode::OK,
                note_page(
                    "Error Reading File",
                    "",
                    &format!("Could not read file {str_path}: '{err}'"),
                ),
            )
                .into_response();

    });

    (headers, buf).into_response()}

async fn get_md(path: Uri) -> Response<axum::body::Body> {
    let mut headers = HeaderMap::new();
    // TODO: query file cache first
    headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let file = match tokio::fs::File::open(&str_path).await {
        Ok(file) => file,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                note_page(
                    "Error 404, File not Found",
                    "",
                    &format!("Could not find file {str_path}"),
                ),
            )
                .into_response();
        }
    };

    (headers, compile_markdown(file, &str_path).await).into_response()
}

async fn compile_markdown(mut file: tokio::fs::File, fname: &str) -> Html<String> {
    let mut contents = String::new();
    file.read_to_string(&mut contents).await.unwrap();

    let mut pdf_file = None;
    let (mod_contents, title): (&str, String) = if contents.starts_with("---") {
        // has yaml frontmatter
        // try to find end of frontmatter
        let mut parts = contents.split("---\n");
        let _empty = parts.next().expect("metadata is present");
        // TODO: handle empty metadata
        let title: String = match parts.next().and_then(|x| serde_yaml::from_str(x).ok()) {
            None => fname.to_string(),
            Some(PaperMeta { tags: _tags, bibtex, pdf}) => {
                pdf_file = Some(pdf);
                format!("Note for Paper: {}", &bibtex.title)
            }
        };


        (parts.next().unwrap_or(""), title)
    } else {
        (contents.as_str(), fname.to_string())
    };



    let parser = Parser::new_ext(&mod_contents, Options::all());
    let mut html_output = String::new();

    fn transform_url(url: CowStr) -> CowStr {
        if url.starts_with("http") {
            return url;
        }

        if url.starts_with("/") || url.starts_with("#") {
            return url;
        }

        CowStr::from(format!("/{}", url))
    }

    pulldown_cmark::html::push_html(&mut html_output, parser.map(|e| {
        use pulldown_cmark::{Event, Tag};
        match e {
            Event::Start(Tag::Link(ltype, mut url, title)) => {
                url = transform_url(url);
                Event::Start(Tag::Link(ltype, url, title))
            },
            Event::End(Tag::Link(ltype, mut url, title)) => {
                url = transform_url(url);
                Event::End(Tag::Link(ltype, url, title))
            }
            other => other
        }
    }));


    note_page(
        &format!("{title}"),
        &format!("<a href=\"/\"><img width=\"32\" height=\"32\" src=\"/favicon.ico\"></a>{}", 
        if let Some(pdf) = pdf_file {
            format!("\n Note for <a href={pdf}>{pdf}</a>", pdf=pdf.to_str().unwrap().to_string())
        } else { String::new() }),
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
        .route("/*path", get(get_file)); // TODO: handle links with tags

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
    papers.push_str("<h2>Daily Notes</h2>");
    papers.push_str(
        "<div style=\"height:10vh;width:100%;overflow:scroll;auto;padding-top:10px;\">\n"
    );
    papers.push_str("<ul class=\"mcol_ul\" id=\"daily\">\n");
    for (_path, note) in NOTES.lock().unwrap().daily.iter() {
        papers.push_str(
            &format!("<li><a href=\"{}\">{}</a></li>", 
                    note.path.strip_prefix(NOTES_PATH).unwrap().display(), 
                    note.path.file_name().unwrap().to_str().unwrap())
        )
    }
    papers.push_str("</ul>\n</div>");
    note_page("Envy - Note Viewer", "", &papers)
}

async fn root() -> Html<String> {
    index_page().await
}
