use axum::http::Uri;
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use chrono::Datelike;
use clap::{Parser, Subcommand};
use envy::file_tokenizer::{self, Lexer};
use itertools::Itertools;
use mupdf::TextPageOptions;
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek};
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
    Citations {
        #[arg(long, short, help = "tag to find citations by")]
        tag: Option<String>,
    },
    #[command(
        about = "Create new note with attached paper",
        long_about = "create a new note in '<notes_root>/<location>/' with bibtex info in system clipboard"
    )]
    NewPaper {
        #[arg(long, short, help = "relative path for new-paper note location",
              default_value_t=String::from("papers"))]
        location: String,
    },
    #[command(about = "Create new daily note")]
    Today,
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

pub fn open_in_editor(path: impl AsRef<Path>) {
    let editor = std::env::vars()
        .find(|(k, _)| k == "EDITOR")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| {
            eprintln!("Not opening file because $EDITOR is not set.");
            std::process::exit(1);
        });
    let _ = std::process::Command::new(editor)
        .args(path.as_ref().to_str())
        .status()
        .expect("Could not start editor");
}

fn main() {
    let mut args = Args::parse();
    args.notes_root = shellexpand::tilde(&args.notes_root).to_string();
    match args.cmd {
        Action::Serve => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(serve(&args.notes_root)),
        Action::Citations { tag } => tokio::runtime::Builder::new_current_thread()
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
                    .filter(|meta| {
                        let Some(tag) = tag.as_ref() else { return true };

                        let Some(tags) = meta.tags.as_ref() else {
                            return false;
                        };

                        tags.contains(&tag)
                    })
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
            let _ = std::env::set_current_dir(&args.notes_root)
                .map_err(|err| eprintln!("Could not change working directory: {err}"));
            open_in_editor(created_file);
        }
        Action::Today => {
            let (new_note, daily_path, datetime) = prep_today(&args.notes_root);
            let _ = std::env::set_current_dir(&args.notes_root)
                .map_err(|err| eprintln!("Could not change working directory: {err}"));
            if !new_note.exists() {
                new_today(new_note.clone(), daily_path, &args.notes_root, datetime)
                    .map_err(|err| {
                        eprintln!("Could not create new daily note: {err}");
                        std::process::exit(1);
                    })
                    .unwrap();
            }
            open_in_editor(&new_note);
        }
    }
}

fn last_entry(daily_path: impl AsRef<Path>, root_path: impl AsRef<Path>) -> Option<String> {
    let mut newest = None;
    let mut newest_path = None;
    for f in std::fs::read_dir(daily_path).ok()?.filter_map(|f| f.ok()) {
        let p = f.path();
        if p.extension().map(|x| x.to_str().expect("is utf8") == "md") != Some(true) {
            continue;
        }

        let Some(f) = p.file_name().expect("file does not end in '..'").to_str() else {
            continue;
        };

        let Ok(date) = chrono::NaiveDate::parse_from_str(f, "%y-%m-%d.md") else {
            continue;
        };

        if let Some(d) = newest {
            if d < date {
                newest = Some(date);
                newest_path = Some(p);
            }
        } else {
            newest = Some(date);
            newest_path = Some(p);
        }
    }

    newest_path.map(|x| {
        x.strip_prefix(root_path)
            .expect("is child of daily_path")
            .to_str()
            .expect("is utf8")
            .to_owned()
    })
}

fn write_cal(
    file: &mut impl std::io::Write,
    datetime: chrono::DateTime<chrono::Local>,
) -> io::Result<()> {
    let date = datetime.date_naive();
    let day_no = date.day0() + 1;

    let month_begin = date.with_day(1).expect("date in range");
    let w = month_begin.weekday().num_days_from_monday() as u8;

    const WEEKDAYS_HEADER: &'static str = " Mon  Tue  Wed  Thu  Fri  Sat  Sun";
    let mon_yr = date.format("%B %Y").to_string();
    let begin = (WEEKDAYS_HEADER.len() - mon_yr.len()) / 2;
    writeln!(file, "```")?;
    writeln!(file, "{}{mon_yr}", " ".repeat(begin))?;
    writeln!(file, "{}", WEEKDAYS_HEADER)?;
    for _ in 0..w {
        write!(file, "     ")?;
    }
    for i in 1..date.num_days_in_month() + 1 {
        if i == day_no as u8 {
            write!(file, " [{i:2}]")?;
        } else {
            write!(file, "{i:>4} ")?;
        }
        if (i + w) % 7 == 0 {
            write!(file, "\n")?;
        }
    }
    writeln!(file, "\n```\n")?;
    Ok(())
}

fn prep_today(root: &str) -> (PathBuf, PathBuf, chrono::DateTime<chrono::Local>) {
    let datetime = chrono::Local::now();
    let fname = format!("{}.md", datetime.format("%y-%m-%d"));

    let daily_path = Path::new(root).join("daily");
    let path = daily_path.join(fname);

    (path, daily_path, datetime)
}

fn new_today(
    new_file_path: PathBuf,
    daily_path: PathBuf,
    root_path: impl AsRef<Path>,
    datetime: chrono::DateTime<chrono::Local>,
) -> Result<(), String> {
    use std::io::{BufWriter, Write};

    let l = last_entry(daily_path, &root_path);

    if let Some(ref l) = l {
        let l = root_path.as_ref().join(l);
        let mut note_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&l)
            .map_err(|err| {
                format!(
                    "Could not update last note '{l}'. File could not be read: {err}.",
                    l = l.display()
                )
            })?;
        let mut note_contents = String::new();
        note_file
            .read_to_string(&mut note_contents)
            .map_err(|err| {
                format!(
                    "Could not update last note '{l}'. File could not be read: {err}.",
                    l = l.display()
                )
            })?;
        let relpath = new_file_path
            .strip_prefix(&root_path)
            .expect("new file is in root")
            .to_str()
            .expect("is utf8");

        note_file.seek(std::io::SeekFrom::Start(0)).map_err(|err| {
            format!(
                "Could not update last note '{l}'. Failed to seek to beginning of file: {err}",
                l = l.display()
            )
        })?;
        let new_note = note_contents.replace("[next](<empty>)", &format!("[next]({relpath})"));
        // only overwrite when necessary
        if new_note != note_contents {
            let n = note_file.write(new_note.as_bytes()).map_err(|err| {
                format!(
                    "Could not update last note '{l}': Failed to write file contents: {err}",
                    l = l.display()
                )
            })?;
            assert_eq!(
                new_note.as_bytes().len(),
                n,
                "Number of bytes written must be equal to length of new file contents"
            );
        }
    }

    let mut file = std::fs::File::create_new(&new_file_path)
        .map(|f| BufWriter::new(f))
        .map_err(|err| {
            format!(
                "Failed to create file '{path}': {err}",
                path = new_file_path.display()
            )
        })?;

    write_cal(&mut file, datetime).map_err(|err| {
        format!(
            "Could not write to file {p}: {err}",
            p = new_file_path.display()
        )
    })?;

    let _ = write!(
        file,
        r#"# {n}

{l}
[next](<empty>)
"#,
        n = datetime.format("%d.%m.%y"),
        l = l.map(|x| format!("[last]({x})")).unwrap_or("".to_string())
    )
    .map_err(|err| {
        format!(
            "could not write to file: '{path}': {err}",
            path = new_file_path.display()
        )
    })?;

    Ok(())
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
