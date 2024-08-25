use std::io::{BufReader, Read};
use std::path::PathBuf;

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
    pub fn with_meta(&self) -> Self {
        let mut ret = self.clone();
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
}
