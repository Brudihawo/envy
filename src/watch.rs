use crate::state::Envy;
use notify::{Event, EventKind};
use tokio::runtime::Builder;

pub fn watch(res: Result<Event, notify::Error>, mut envy: Envy) {
    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    match res {
        Ok(Event {
            kind: EventKind::Modify(modkind),
            paths,
            ..
        }) => {
            use notify::event::ModifyKind::*;
            match modkind {
                Any | Other => todo!("handle unknown fs event"),
                Data(_) | Metadata(_) => {
                    let path = paths.first().unwrap();
                    rt.block_on(envy.update_file(path));
                }
                Name(notify::event::RenameMode::Both) => {
                    let from = paths.first().unwrap();
                    let to = paths.last().unwrap();
                    rt.block_on(envy.move_file(from, to));
                }
                Name(_) => (),
            }
        }
        Ok(_) => (),
        Err(err) => eprintln!("Error processing file event: {err}"),
    };
}
