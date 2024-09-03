use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, HeaderMap, Response, Uri};
use axum::response::IntoResponse;
use tokio::io::AsyncReadExt;

use crate::file::File;
use crate::file_requests::{file_error_page, file_or_err_page, get_md, note_page, NOTES_PATH};

macro_rules! serve_font {
    ($font_name:literal, $identifier:ident, $mtype:ident) => {
        let mut headers = HeaderMap::new();
        if $identifier.ends_with($font_name) {
            let file_contents = include_bytes!(concat!(
                "../vendor/mathjax/output/chtml/fonts/woff-v2/",
                $font_name
            ));
            headers.insert(header::CONTENT_TYPE, $mtype.parse().unwrap());
            return Ok((headers, file_contents).into_response());
        }
    };
}

type NoteMap = HashMap<String, File>;
#[derive(Clone)]
pub struct Envy {
    notes: Arc<Mutex<HashMap<String, NoteMap>>>,
}

impl Envy {
    pub async fn build_database(path: &str) -> Self {
        use walkdir::WalkDir;
        let files = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.map_or(None, |ok| Some(ok)))
            .filter(|e| {
                let m = e.metadata().expect("entry metadata is readable");
                let ext = e.path().extension();
                if ext.is_some() {
                    m.is_file() && ext.unwrap() == "md"
                } else {
                    false
                }
            })
            .map(|e| async move { File::new(e.path().to_string_lossy().to_string()) });

        let mut notes: HashMap<String, NoteMap> = HashMap::new();
        for file in files {
            let file = file.await.await;
            let parent = file
                .path
                .parent()
                .expect("we should not be running on '/'")
                .to_string_lossy()
                .to_string();

            if let Some(sub_notes) = notes.get_mut(&parent) {
                let note_path = file.path.to_str().expect("note path is convertible to str");
                if let Some(note) = sub_notes.get_mut(note_path) {
                    if note.modified < file.modified {
                        *note = file;
                    }
                } else {
                    sub_notes.insert(file.path.to_str().unwrap().to_string(), file);
                }
            } else {
                notes.insert(parent.clone(), HashMap::new());
                notes
                    .get_mut(&parent)
                    .expect("we just inserted this")
                    .insert(file.path.to_str().unwrap().to_string(), file);
            }
        }

        Envy {
            notes: Arc::new(Mutex::new(notes)),
        }
    }

    async fn get_pdf(&self, path: Uri) -> Result<Response<Body>, Response<Body>> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());

        let str_path = format!("{NOTES_PATH}{str_path}", str_path = path.to_string());

        let mut file = file_or_err_page(&str_path).await?;
        let mut buf = Vec::new();
        let _ = file
            .read_to_end(&mut buf)
            .await
            .map_err(|err| file_error_page("Error Reading File", &str_path, err));

        Ok((headers, buf).into_response())
    }

    pub async fn get_file(&self, path: Uri) -> Result<Response<Body>, Response<Body>> {
        let p = path.path();
        // TODO: query file cache first

        if p.ends_with(".pdf") {
            return self.get_pdf(path).await;
        }
        if p.ends_with(".md") {
            return get_md(path).await;
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

    pub fn render_index_page(&self) -> impl IntoResponse {
        let mut page = String::new();
        let notes = self.notes.lock().unwrap();
        let _ = writeln!(&mut page, "<div class='tabbed'>");
        let mut first = true;
        let keys_w_display: Vec<_> = notes
            .iter()
            .map(|(path, _)| path)
            .sorted()
            .map(|parent| {
                let mut nice_parent = parent.strip_prefix(NOTES_PATH).unwrap().to_string();
                nice_parent = nice_parent.trim_start_matches('/').to_string();
                if nice_parent.is_empty() {
                    nice_parent.push_str("Root")
                }

                (parent, nice_parent)
            })
            .collect();
        let _ = writeln!(&mut page, "<div id='tabbed-radios'>");
        for (parent, _) in keys_w_display.iter() {
            let _ = writeln!(
                &mut page,
                "<input type='radio' id='{parent}-radio' name='tabs' onclick='update_radios()' parent='{parent}' {chk}>",
                chk = if first { "checked" } else { "" }
            );
            first = false;
        }
        let _ = writeln!(&mut page, "</div>");

        let _ = writeln!(&mut page, "<span class='title'><a class='site_icon' href=\"/\"><img width=\"48\" height=\"48\" src=\"/favicon.ico\"></a>");
        let _ = writeln!(&mut page, "<ul class='tabs'>");
        for (parent, nice_parent) in keys_w_display.iter() {
            let _ = writeln!(
                &mut page,
                "<li class='tab' id='{parent}-tab'><label for='{parent}-radio'><strong>{nice_parent}</strong></label></li>",
            );
        }
        let _ = writeln!(&mut page, "</ul></span>");

        for (parent, _) in keys_w_display.iter() {
            let note_map = notes.get(*parent).expect("we use the keys we got before");
            let _ = writeln!(&mut page, "<div class='tab-content' id='{parent}-content' parent='{parent}'>");
            let _ = writeln!(&mut page, "  <ul id='{parent}-ul'>");

            for (_path, note) in note_map.iter().sorted_by_key(|&(path, _)| path) {
                let _ = write!(&mut page, "    ");
                note.write_index_entry(&mut page, NOTES_PATH);
            }

            let _ = writeln!(&mut page, "  </ul>");
            let _ = writeln!(&mut page, "</div>");
        }
        let _ = writeln!(&mut page, "</div>");

        note_page("Envy - Note Viewer", "", &page)
    }
}
