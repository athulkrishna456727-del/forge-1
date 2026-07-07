use axum::{
    routing::post,
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct RunRequest {
    pub task: String,
    pub model: Option<String>,
    pub max_steps: Option<u32>,
    pub effort: Option<String>,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub result: String,
    pub steps_taken: u32,
}

async fn run_handler(
    State(valid_keys): State<Arc<HashSet<String>>>,
    headers: HeaderMap,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let provided_key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !valid_keys.contains(provided_key) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid or missing API key".to_string()));
    }

    let (model, max_steps) = if let Some(effort) = &req.effort {
        let cfg = crate::effort_config(effort);
        (cfg.model, cfg.max_steps)
    } else {
        (
            req.model.clone().unwrap_or_else(|| "qwen2.5-coder:3b".to_string()),
            req.max_steps.unwrap_or(8),
        )
    };

    match crate::run_agent(&req.task, &model, max_steps).await {
        Ok((result, steps_taken)) => Ok(Json(RunResponse { result, steps_taken })),
        Err(e) => Ok(Json(RunResponse {
            result: format!("Error: {}", e),
            steps_taken: 0,
        })),
    }
}

pub fn create_router(valid_keys: Arc<HashSet<String>>) -> Router {
    Router::new()
        .route("/v1/run", post(run_handler))
        .with_state(valid_keys)
}