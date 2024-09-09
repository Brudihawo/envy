use crate::state::ServerState;
use axum::extract::Query;
use axum::response::Html;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Deserialize, Serialize, Debug)]
pub struct Search {
    pub any: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct QueryResult {
    pub file: String,
}

pub async fn query_meta(query: Query<Search>, envy: ServerState) -> Html<String> {
    let text = &query.any;
    // server side rendering for now
    let mut h = String::new();
    let query_res = envy.query_any(text);
    if let Some(query_res) = query_res {
        let _ = writeln!(&mut h, "<ul>");
        for (_, elem) in query_res {
            let _ = writeln!(&mut h, "  {elem}");
        }
        let _ = writeln!(&mut h, "</ul>");
    } else {
        let _ = writeln!(&mut h, "<h3 style='position: absolute; display: flex; align-content: center;width: 100%; justify-content:center;'>No Results</h3>");
    }
    h.into()
}

pub async fn query_fulltext(query: Query<Search>, envy: ServerState) -> Html<String> {
    let text = &query.any;
    // server side rendering for now
    let mut h = String::new();
    let query_res = envy.query_fulltext(text);
    if let Some(query_res) = query_res {
        let _ = writeln!(&mut h, "<ul>");
        for (_, elem) in query_res {
            let _ = writeln!(&mut h, "  {elem}");
        }
        let _ = writeln!(&mut h, "</ul>");
    } else {
        let _ = writeln!(&mut h, "<h3 style='position: absolute; display: flex; align-content: center;width: 100%; justify-content:center;'>No Results</h3>");
    }
    h.into()
}
