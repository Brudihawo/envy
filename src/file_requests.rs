use crate::file::PaperMeta;
use axum::body::Body;
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use cfg_if::cfg_if;
use pulldown_cmark::{CowStr, Options, Parser};
use tokio::io::AsyncReadExt;

pub const MATHJAX_CFG: &'static str = include_str!("./mathjax_cfg.js");
pub const MATHJAX_URI: &'static str = "/vendor/mathjax/tex-chtml.js";
pub const NOTES_PATH: &'static str = "/home/hawo/notes";

pub fn note_page(title: &str, body_pre: &str, body: &str) -> Html<String> {
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
<div id='search_parent' class='search'><div class='cover'></div><div class='search_in'><input id='search_input' type='text' onkeyup='update_search()' placeholder='search...'/><div id='search_res_div' class='search_res'></div></div></div>
<script type='text/javascript'>
    update_tab_display();
    document.addEventListener('keydown', process_down, false);
    document.addEventListener('keyup', process_up, false);
</script>
</body>
</html>"
    )
    .into()
}

pub fn file_error_page<T>(msg: &str, path: &str, err: T) -> Response<Body>
where
    T: std::error::Error,
{
    (
        StatusCode::OK,
        note_page(msg, "", &format!("{msg} {path}: '{err}'")),
    )
        .into_response()
}

pub async fn file_or_err_page(path: &str) -> Result<tokio::fs::File, Response<Body>> {
    tokio::fs::File::open(&path)
        .await
        .map_err(|err| file_error_page("Error Opening File:", &path, err))
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
            Some(PaperMeta {
                tags: _tags,
                bibtex,
                pdf,
            }) => {
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

    pulldown_cmark::html::push_html(
        &mut html_output,
        parser.map(|e| {
            use pulldown_cmark::{Event, Tag};
            match e {
                Event::Start(Tag::Link(ltype, mut url, title)) => {
                    url = transform_url(url);
                    Event::Start(Tag::Link(ltype, url, title))
                }
                Event::End(Tag::Link(ltype, mut url, title)) => {
                    url = transform_url(url);
                    Event::End(Tag::Link(ltype, url, title))
                }
                other => other,
            }
        }),
    );

    note_page(
        &format!("{title}"),
        &format!(
            "<a href=\"/\"><img width=\"32\" height=\"32\" src=\"/favicon.ico\"></a>{}",
            if let Some(pdf) = pdf_file {
                format!(
                    "\n Note for <a href={pdf}>{pdf}</a>",
                    pdf = pdf.to_str().unwrap().to_string()
                )
            } else {
                String::new()
            }
        ),
        &html_output,
    )
}

pub async fn get_md(path: Uri) -> Result<Response<Body>, Response<Body>> {
    let mut headers = HeaderMap::new();
    // TODO: query file cache first
    headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

    let file = file_or_err_page(&str_path).await?;
    Ok((headers, compile_markdown(file, &str_path).await).into_response())
}

pub async fn style() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());

    cfg_if! {
        if #[cfg(reload_css)] {
            let content: str = include_str!("style.css");
        } else {
            let mut file = tokio::fs::File::open("src/style.css").await.unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).await.unwrap();
        }
    }

    (headers, content)
}

pub async fn favicon() -> impl IntoResponse {
    let bytes = include_bytes!("../assets/favicon.ico");
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        r#"attachment; filename="favicon.ico"#.parse().unwrap(),
    );

    (headers, bytes)
}

pub async fn script() -> Result<Response<Body>, Body> {
    let file_contents = include_str!("script.js");
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/javascript".parse().unwrap());
    Ok((headers, file_contents).into_response())
}

