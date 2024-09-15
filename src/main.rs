use axum::http::Uri;
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use envy::api::{query_fulltext, query_meta};
use envy::file_requests::{favicon, script, style};
use envy::state::{Envy, ServerState};
use envy::watch::watch;
use notify::{recommended_watcher, RecursiveMode, Watcher};

use tracing_subscriber;

async fn index(nvy: ServerState) -> impl IntoResponse {
    nvy.render_index_page()
}

async fn file(nvy: ServerState, uri: Uri) -> impl IntoResponse {
    nvy.get_file(uri).await
}

#[derive(Subcommand)]
enum Action {
    #[command(
        about = "Serve notes",
        long_about = "Serve the notes database on http://localhost:6969"
    )]
    Serve,
    #[command(
        about = "Generate citations list",
        long_about = "Generate a list of all the bibtex citations present in the notes DB"
    )]
    Citations,
    #[command(
        about = "Create new note",
        long_about = "create a new note in '<notes_root>/<location>/' with bibtex info in system clipboard"
    )]
    NewPaper {
        #[arg(long, short, help = "relative path for new-paper note location",
              default_value_t=String::from("papers"))]
        location: String,
    },
}

pub fn new_paper(root: &str, location: &str) -> Result<PathBuf, String> {
    use std::io::{BufWriter, Write};
    let clip =
        cli_clipboard::get_contents().map_err(|err| format!("Failed to get clipboard: {err}"))?;

    let entry = envy::bibtex::BibtexEntry::try_from_str(&clip)
        .map_err(|err| format!("Failed to parse bibtex entry: {err}"))?;

    let path = Path::new(root)
        .join(location)
        .join(format!("{name}.md", name = entry.name));

    let mut file = std::fs::File::create_new(&path)
        .map(|f| BufWriter::new(f))
        .map_err(|err| {
            format!(
                "Failed to create file '{path}': {err}",
                path = path.display()
            )
        })?;

    let _ = write!(
        file,
        r#"---
bibtex: "{entry}"
pdf: "./doc/{name}.pdf"
tags: [unread]
---

# {title}"#,
        name = entry.name,
        title = entry.title
    )
    .map_err(|err| {
        format!(
            "could not write to file: '{path}': {err}",
            path = path.display()
        )
    })?;

    Ok(path)
}

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(default_value_t=String::from("~/notes"))]
    notes_root: String,
    #[command(subcommand)]
    cmd: Action,
}

pub fn main() {
    let mut args = Args::parse();
    args.notes_root = shellexpand::tilde(&args.notes_root).to_string();
    match args.cmd {
        Action::Serve => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(serve(&args.notes_root)),
        Action::Citations => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let envy = Envy::build_database(&args.notes_root).await;
                for entry in envy
                    .notes
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(_loc, sub)| sub.iter())
                    .flatten()
                    .filter_map(|(_path, file)| file.meta.as_ref())
                    .map(|f| &f.bibtex)
                {
                    println!("{}", entry)
                }
            }),
        Action::NewPaper { location } => {
            let created_file = new_paper(&args.notes_root, &location)
                .map_err(|err| {
                    eprintln!("Could not create new paper note: {err}");
                    std::process::exit(1);
                })
                .unwrap();
            let editor = std::env::vars()
                .find(|(k, _)| k == "EDITOR")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| {
                    eprintln!("Not opening file because $EDITOR is not set.");
                    std::process::exit(1);
                });
            let _ = std::process::Command::new(editor)
                .args(created_file.to_str())
                .status()
                .expect("Could not start editor");
        }
    }
}
//
// #[tokio::main]
async fn serve(loc: &impl AsRef<Path>) {
    tracing_subscriber::fmt::init();

    let envy = Envy::build_database(loc).await;

    let nvy_watch = envy.clone();
    let mut watcher = recommended_watcher(move |res| watch(res, nvy_watch.clone()))
        .expect("could not create fs watcher");

    watcher
        .watch(loc.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    let app = Router::new()
        .route("/", get(index).with_state(envy.clone()))
        .route("/script.js", get(script))
        .route("/style.css", get(style))
        .route("/favicon.ico", get(favicon))
        .route("/api/meta", get(query_meta).with_state(envy.clone()))
        .route(
            "/api/fulltext",
            get(query_fulltext).with_state(envy.clone()),
        )
        .route("/*path", get(file).with_state(envy.clone()));
    // TODO: handle links with tags

    let address = "localhost:6969";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    println!("Serving at http://{address}");
    axum::serve(listener, app).await.unwrap();
}
