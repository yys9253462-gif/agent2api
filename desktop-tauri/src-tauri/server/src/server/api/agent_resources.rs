use axum::{body::Bytes, extract::{Path, Query, State}, response::Response};
use serde_json::Value;
use std::collections::HashMap;

use crate::server::{core::agents, errors::management_error, http::{ok_json, parse_body}, ServerState};

fn response(result: Result<Value, String>) -> Response {
    match result { Ok(value) => ok_json(value), Err(error) => management_error(if error.contains("已被其他") || error.contains("已变化") { 409 } else { 400 }, error) }
}

pub async fn get(State(_state): State<ServerState>, Path(id): Path<String>, Query(query): Query<HashMap<String, String>>) -> Response {
    response((|| {
        let def = agents::find(&id).ok_or("未知客户端")?;
        let kind = query.get("kind").map(String::as_str).unwrap_or("prompt");
        if kind == "skill" { agents::resources::skill(def, query.get("id").map(String::as_str).unwrap_or("")) }
        else { agents::resources::snapshot(def, kind) }
    })())
}

pub async fn preview(State(_state): State<ServerState>, Path(id): Path<String>, body: Bytes) -> Response {
    response((|| {
        let payload = parse_body(&body).map_err(|error| error.message)?;
        let def = agents::find(&id).ok_or("未知客户端")?;
        agents::resources::create_preview(def, &payload)
    })())
}

pub async fn apply(State(_state): State<ServerState>, Path(id): Path<String>, body: Bytes) -> Response {
    response((|| {
        let payload = parse_body(&body).map_err(|error| error.message)?;
        let def = agents::find(&id).ok_or("未知客户端")?;
        agents::resources::apply(def, &payload)
    })())
}
