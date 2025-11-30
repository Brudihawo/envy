use std::fmt::Write;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::bibtex::BibtexEntry;
use crate::file_tokenizer::Lexer;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct PaperMeta {
    pub tags: Option<Vec<String>>,
    pub bibtex: BibtexEntry,
    pub pdf: PathBuf,
}

#[derive(Clone, Debug)]
pub struct File {
    pub modified: std::time::SystemTime,
    pub path: std::path::PathBuf,
    pub content: String,
    pub num_words: usize,
    pub tf_map: HashMap<String, usize>,
    pub meta: Option<PaperMeta>,
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.modified == other.modified
    }
}

impl Eq for File {}

impl std::hash::Hash for File {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.path.as_os_str().to_str().unwrap().as_bytes());
        self.modified.hash(state)
    }
}

fn load_pdf_text_to_tf_map(
    pdf_path: &Path,
    tf_map: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let doc = mupdf::Document::open(pdf_path)
        .map_err(|err| format!("Could not open file '{p}': {err}", p = pdf_path.display()))?;

    let mut text = String::new();
    let n = doc.page_count().map_err(|err| {
        format!(
            "Could not get number of pages in pdf '{p}': {err}",
            p = pdf_path.display()
        )
    })?;
    if n > 100 {
        eprintln!(
            "Skipping file '{p}' during full-text indexing. Too long ({n} pages).",
            p = pdf_path.display()
        );

        return Ok(());
    };
    for p in doc.pages().map_err(|err| {
        format!(
            "Could not get pages for file '{p}': {err}",
            p = pdf_path.display()
        )
    })? {
        match p {
            Ok(p) => text.write_str(&p.to_text().unwrap()).unwrap(),
            Err(err) => println!("ERROR: Could not load page: {err}"),
        }
    }
    Lexer::new(&text).token_frequencies_into_existing_map(tf_map);
    Ok(())
}

impl File {
    pub async fn new(path: impl AsRef<Path>) -> Self {
        let file = tokio::fs::File::open(&path).await.expect("file exists");
        let metadata = file.metadata().await.expect("file has readable metadata");

        let mut content = String::new();
        BufReader::new(std::fs::File::open(&path).unwrap())
            .read_to_string(&mut content)
            .unwrap();

        let meta = if content.starts_with("---") {
            // has yaml frontmatter
            // try to find end of frontmatter
            let mut parts = content.split("---\n");
            let _empty = parts.next().expect("metadata is present");
            let meta = parts.next().expect("metadata is present");
            let meta: Option<PaperMeta> = serde_yaml::from_str(meta)
                .map_err(|err| {
                    // TODO: proper logging
                    eprintln!(
                        "Invalid Metadata in {path}: '{err}' (read metadata: {meta})",
                        path = path.as_ref().display()
                    );
                    ()
                })
                .ok();
            meta
        } else {
            None
        };

        let f = tokio::fs::read_to_string(&path).await.unwrap();
        let mut tf_map = HashMap::new();
        Lexer::new(&f).token_frequencies_into_existing_map(&mut tf_map);
        if let Some(ref m) = meta {
            let parent = path.as_ref().parent().expect("note file has parent");
            let pdf_path = parent.join(&m.pdf);
            if let Err(err) = load_pdf_text_to_tf_map(&pdf_path, &mut tf_map) {
                eprintln!(
                    "ERROR: Could not get data for pdf (at '{pdf_path}') of file '{p}'. {err}",
                    pdf_path = pdf_path.display(),
                    p = path.as_ref().display()
                )
            }
        }

        Self {
            path: path.as_ref().to_path_buf(),
            modified: metadata.modified().expect("can get mtime"),
            content,
            num_words: tf_map.iter().map(|(_, count)| count).sum(),
            tf_map,
            meta,
        }
    }

    pub fn write_index_entry(
        &self,
        page: &mut impl Write,
        base: impl AsRef<Path>,
        with_parent: bool,
    ) {
        let meta = self.meta.as_ref();
        let fname = self.path.file_name().unwrap().to_str().unwrap();
        let path = self.path.strip_prefix(base).unwrap().display();

        let dpath = if with_parent {
            path.to_string()
        } else {
            fname.to_string()
        };

        if let Some(meta) = meta {
            let _ = writeln!(page, "<li><strong>{title}</strong><br/>{year} <em>{authors}</em><br/><a href=\"/{path}\">{dpath}</a></li>", 
                        title=meta.bibtex.title,
                        authors=meta.bibtex.author,
                        year=meta.bibtex.year,
                    );
        } else {
            let _ = writeln!(page, "<li><a href='/{path}'>{dpath}</a></li>",);
        }
    }

    pub fn matches_any(&self, any: &str, prefix: impl AsRef<Path>) -> u32 {
        let mut match_case = false;
        for c in any.chars() {
            if c.is_uppercase() {
                match_case = true;
            }
        }
        let any_lower = any.to_lowercase();
        let any = if !match_case { &any_lower } else { any };

        let mut score: u32 = 0;
        if let Some(meta) = &self.meta {
            if match_case {
                score += meta.bibtex.name.contains(any) as u32;
                score += (meta.bibtex.year == any) as u32 * 5;
                score += meta.bibtex.title.contains(any) as u32 * 2;
                score += meta.bibtex.author.contains(any) as u32 * 2;
            } else {
                score += meta.bibtex.name.to_lowercase().contains(any) as u32;
                score += (meta.bibtex.year == any) as u32 * 5;
                score += meta.bibtex.title.to_lowercase().contains(any) as u32 * 2;
                score += meta.bibtex.author.to_lowercase().contains(any) as u32 * 2;
            }
        }

        score += self
            .path
            .strip_prefix(prefix)
            .unwrap()
            .to_str()
            .expect("path is valid unicode")
            .contains(any) as u32
            * 3;

        return score;
    }
}

pub fn tags_arr(in_tags: &[String]) -> String {
    let mut tags = String::from("[");

    for (i, tag) in in_tags.iter().enumerate() {
        if i > 0 {
            tags.push_str(",");
        }
        tags.push_str(tag);
    }
    tags.push_str("]");

    tags
}
