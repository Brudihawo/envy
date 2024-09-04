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

    pub fn write_index_entry(&self, page: &mut impl Write, base: &str, with_parent: bool) {
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

    pub fn matches_any(&self, any: &str, prefix: &str) -> u32 {
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
