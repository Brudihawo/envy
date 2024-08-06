use axum::body::Body;
use axum::http::{HeaderMap, header, Uri, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use once_cell::sync::Lazy;
use pulldown_cmark::{Options, Parser, CowStr};
use tokio::io::AsyncReadExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::file::File;
use crate::file::PaperMeta;

#[derive(Default)]
pub struct Notes {
    pub papers: HashMap<String, File>,
    pub daily: HashMap<String, File>,
    pub other: HashMap<String, File>,
}

pub static NOTES: Lazy<Arc<Mutex<Notes>>> = Lazy::new(|| Arc::new(Mutex::new(Notes::default())));
pub async fn favicon() -> impl IntoResponse {
    let bytes = include_bytes!("../assets/favicon.ico");
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"favicon.ico\"".parse().unwrap(),
    );

    (headers, bytes)
}

pub async fn root() -> Html<String> {
    index_page().await
}


macro_rules! serve_font {
    ($font_name:literal, $identifier:ident, $mtype:ident) => {
        let mut headers = HeaderMap::new();
        if $identifier.ends_with($font_name) {
            let file_contents = include_bytes!(concat!("../vendor/mathjax/output/chtml/fonts/woff-v2/", $font_name));
            headers.insert(header::CONTENT_TYPE, $mtype.parse().unwrap());
            return Ok((headers, file_contents).into_response());
        }
    }
}

const MATHJAX_CFG: &'static str = include_str!("./mathjax_cfg.js");
const MATHJAX_URI: &'static str = "/vendor/mathjax/tex-chtml.js";
const NOTES_PATH: &'static str = "/home/hawo/notes";

fn note_page(title: &str, body_pre: &str, body: &str) -> Html<String> {
    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\"/>
    <link rel=\"stylesheet\" href=\"/style.css\">
    <script src=\"/script.js\"></script>
    <script>{MATHJAX_CFG}</script>
    <script type=\"text/javascript\" async src=\"{MATHJAX_URI}?config=TeX-AMS-MML_HTMLorMML\"></script>
    <title>{title}</title>
</head>
<body>
    {body_pre}
    {body}
</body>
</html>"
    )
    .into()
}


fn file_error_page<T>(msg: &str, path: &str, err: T) -> Response<Body> 
where T: std::error::Error{
        (StatusCode::OK, note_page(
                msg,
                "",
                &format!("{msg} {path}: '{err}'"),
            )
        ).into_response()
}

async fn file_or_err_page(path: &str) -> Result<tokio::fs::File, Response<Body>>{
    tokio::fs::File::open(&path).await.map_err(|err| 
        file_error_page("Error Opening File:", &path, err))
}

async fn get_pdf(path: Uri) -> Result<Response<Body>, Response<Body>> {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let mut file = file_or_err_page(&str_path).await?;
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf).await.map_err(|err| file_error_page("Error Reading File", &str_path, err)
    );

    Ok((headers, buf).into_response())
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


async fn get_md(path: Uri) -> Result<Response<Body>, Response<Body>> {
    let mut headers = HeaderMap::new();
    // TODO: query file cache first
    headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let file = file_or_err_page(&str_path).await?;
    Ok((headers, compile_markdown(file, &str_path).await).into_response())
}


pub async fn get_file(path: Uri) -> Result<Response<Body>, Response<Body>> {
    let p = path.path();
    // TODO: query file cache first

    if p.ends_with(".pdf") {
        return get_pdf(path).await;
    } 
    if p.ends_with(".md") {
        return get_md(path).await;
    } 
    if p == "/script.js" {
        let file_contents = include_str!("script.js");
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "text/javascript".parse().unwrap());
        return Ok((headers, file_contents).into_response());
    }

    if p == "/vendor/mathjax/tex-chtml.js" {
        let file_contents = include_str!("../vendor/mathjax/tex-chtml.js");
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "text/javascript".parse().unwrap());
        return Ok((headers, file_contents).into_response());
    }


    let mtype = if p.ends_with(".js") {
        "text/javascript".to_string()
    } else if p.ends_with(".woff") {
        "font/woff".to_string()
    } else {
        mime_guess::from_path(p).first().unwrap().to_string()
    };

    if mtype.starts_with("text") {
        let mut headers = HeaderMap::new();
        let mut file_contents = String::new();
        let mut file = file_or_err_page(p).await?;
        file.read_to_string(&mut file_contents).await.unwrap();
        headers.insert(header::CONTENT_TYPE, mtype.parse().unwrap());
        return Ok((headers, file_contents).into_response());
    }


    serve_font!("MathJax_Math-Italic.woff", p, mtype);
    serve_font!("MathJax_AMS-Regular.woff", p, mtype);
    serve_font!("MathJax_Calligraphic-Bold.woff", p, mtype);
    serve_font!("MathJax_Calligraphic-Regular.woff", p, mtype);
    serve_font!("MathJax_Fraktur-Bold.woff", p, mtype);
    serve_font!("MathJax_Fraktur-Regular.woff", p, mtype);
    serve_font!("MathJax_Main-Bold.woff", p, mtype);
    serve_font!("MathJax_Main-Italic.woff", p, mtype);
    serve_font!("MathJax_Main-Regular.woff", p, mtype);
    serve_font!("MathJax_Math-BoldItalic.woff", p, mtype);
    serve_font!("MathJax_Math-Italic.woff", p, mtype);
    serve_font!("MathJax_Math-Regular.woff", p, mtype);
    serve_font!("MathJax_SansSerif-Bold.woff", p, mtype);
    serve_font!("MathJax_SansSerif-Italic.woff", p, mtype);
    serve_font!("MathJax_SansSerif-Regular.woff", p, mtype);
    serve_font!("MathJax_Script-Regular.woff", p, mtype);
    serve_font!("MathJax_Size1-Regular.woff", p, mtype);
    serve_font!("MathJax_Size2-Regular.woff", p, mtype);
    serve_font!("MathJax_Size3-Regular.woff", p, mtype);
    serve_font!("MathJax_Size4-Regular.woff", p, mtype);
    serve_font!("MathJax_Typewriter-Regular.woff", p, mtype);
    serve_font!("MathJax_Vector-Bold.woff", p, mtype);
    serve_font!("MathJax_Vector-Regular.woff", p, mtype);
    serve_font!("MathJax_Zero.woff", p, mtype);

    Err("Invalid File".into_response())
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
    papers.push_str("<h2>Paper Notes</h2>\n");
    papers.push_str("<input type=\"text\" id=\"paper_search\" onkeyup=\"filter_list('papers', 'paper_search')\" placeholder=\"Search Tags or Names\">\n");
    papers.push_str("<div style=\"height:50vh;width:100%;overflow:scroll;auto;padding-top:10px;\">\n");
    papers.push_str("<ul id='papers'>");
    for (_path, paper) in NOTES.lock().unwrap().papers.iter() {
        let meta = &paper.meta.as_ref().unwrap();

        let mut tags = String::from("[");
        if let Some(mtags) = &meta.tags {
            for (i, tag) in mtags.iter().enumerate() {
                if i > 0 {
                    tags.push_str(",");
                }
                tags.push_str(tag);
            }
        }
        tags.push_str("]");

        papers.push_str(&format!("<li authors=\"{authors}\" tags=\"{tags}\" title=\"{title}\"><strong>{title}</strong></br>{year} <em>{authors}</em></br><a href=\"{path}\">{fname}</a></li>", 
            title=meta.bibtex.title, 
            authors=meta.bibtex.author, 
            year=meta.bibtex.year, 
            path=paper.path.strip_prefix(NOTES_PATH).unwrap().display(), 
            fname=paper.path.file_name().unwrap().to_str().unwrap()));
    }
    papers.push_str("</ul>\n</div>\n");

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

    papers.push_str("<h2>Other Notes</h2>");
    papers.push_str(
        "<div style=\"height:10vh;width:100%;overflow:scroll;auto;padding-top:10px;\">\n"
    );
    papers.push_str("<ul class=\"mcol_ul\" id=\"daily\">\n");
    for (_path, note) in NOTES.lock().unwrap().other.iter() {
        papers.push_str(
            &format!("<li><a href=\"{}\">{}</a></li>", 
                    note.path.strip_prefix(NOTES_PATH).unwrap().display(), 
                    note.path.file_name().unwrap().to_str().unwrap())
        )
    }
    papers.push_str("</ul>\n</div>");
    note_page("Envy - Note Viewer", "", &papers)
}
