use std::fmt::Write;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::bibtex::BibtexEntry;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct PaperMeta {
    pub tags: Option<Vec<String>>,
    pub bibtex: BibtexEntry,
    pub pdf: PathBuf,
}

#[derive(Clone, Debug)]
pub enum FileContents {
    PaperNote { content: String },
    General { content: String },
    Pdf { bytes: Vec<u8> },
}

#[derive(Clone, Debug)]
pub struct File {
    pub modified: std::time::SystemTime,
    pub path: std::path::PathBuf,
    pub loaded_content: Option<FileContents>,
    pub meta: Option<PaperMeta>,
}

impl File {
    pub async fn new(path: impl AsRef<Path>) -> Self {
        let file = tokio::fs::File::open(&path).await.expect("file exists");
        let metadata = file.metadata().await.expect("file has readable metadata");
        let mut ret = Self {
            path: path.as_ref().to_path_buf(),
            modified: metadata.modified().expect("can get mtime"),
            loaded_content: None,
            meta: None,
        };

        let mut contents = String::new();
        BufReader::new(std::fs::File::open(&ret.path).unwrap())
            .read_to_string(&mut contents)
            .unwrap();

        if contents.starts_with("---") {
            // has yaml frontmatter
            // try to find end of frontmatter
            let mut parts = contents.split("---\n");
            let _empty = parts.next().expect("metadata is present");
            // TODO: handle empty metadata
            let meta = parts.next().expect("metadata is present");
            let meta: PaperMeta = serde_yaml::from_str(meta).expect("Parseable Metadata");
            ret.meta = Some(meta);
        } else {
            ret.meta = None
        }
        ret
    }

    pub fn write_index_entry(&self, page: &mut impl Write, base: &str) {
        let meta = self.meta.as_ref();
        let fname = self.path.file_name().unwrap().to_str().unwrap();
        let path = self.path.strip_prefix(base).unwrap().display();

        if let Some(meta) = meta {
            let tags = if let Some(ref t) = meta.tags {
                tags_arr(t)
            } else {
                String::from("[]")
            };
            let _ = writeln!(page, "<li authors=\"{authors}\" tags=\"{tags}\" title=\"{title}\"><strong>{title}</strong><br/>{year} <em>{authors}</em><br/><a href=\"{path}\">{fname}</a></li>", 
                        title=meta.bibtex.title,
                        authors=meta.bibtex.author,
                        year=meta.bibtex.year,
                    );
        } else {
            let _ = writeln!(page, "<li><a href='{path}'>{fname}</a></li>");
        }
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
