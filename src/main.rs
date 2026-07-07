mod tools;
mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::fs;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String, // tells Ollama to force valid JSON output
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    response: String,
}

// This is the shape we ask the AI to reply in
#[derive(Deserialize, Debug)]
struct AgentAction {
    tool: String,        // "read_file", "write_file", "run_command", or "done"
    path: Option<String>,
    content: Option<String>,
    command: Option<String>,
    reasoning: Option<String>,   // why it chose this action (helps you debug)
}

// --- Section 9 ---
// Maps an effort level to a model + max_steps.
pub struct EffortConfig {
    pub model: String,
    pub max_steps: u32,
}

pub fn effort_config(level: &str) -> EffortConfig {
    match level {
        "low"    => EffortConfig { model: "qwen2.5-coder:3b".to_string(), max_steps: 4 },
        "medium" => EffortConfig { model: "qwen2.5-coder:3b".to_string(), max_steps: 8 },
        "high"   => EffortConfig { model: "qwen2.5-coder:7b".to_string(), max_steps: 12 },
        "ultra"  => EffortConfig { model: "qwen2.5-coder:7b".to_string(), max_steps: 20 },
        _        => EffortConfig { model: "qwen2.5-coder:3b".to_string(), max_steps: 8 }, // default = medium
    }
}
// --- END Section 9 ---

// --- NEW: Section 11 (API key loading) ---
#[derive(Deserialize)]
struct ApiKeysFile {
    keys: Vec<String>,
}

fn load_api_keys() -> Arc<HashSet<String>> {
    let content = fs::read_to_string("api_keys.json")
        .expect("api_keys.json not found — create it in the project root");
    let parsed: ApiKeysFile = serde_json::from_str(&content)
        .expect("api_keys.json is malformed — check the JSON syntax");
    Arc::new(parsed.keys.into_iter().collect())
}
// --- END NEW ---

async fn ask_model(client: &reqwest::Client, task: &str, history: &str, model: &str) -> anyhow::Result<AgentAction> {
    let system_prompt = format!(
        r#"You are a coding agent. You must respond with EXACTLY ONE flat JSON object — never nested, never multiple actions at once.

Valid formats (pick ONE per response):
{{"tool": "read_file", "path": "...", "reasoning": "..."}}
{{"tool": "write_file", "path": "...", "content": "...", "reasoning": "..."}}
{{"tool": "run_command", "command": "...", "reasoning": "..."}}
{{"tool": "done", "reasoning": "..."}}

Task: {task}

History so far:
{history}

IMPORTANT: If the history shows the task has already been completed, respond with {{"tool": "done", "reasoning": "..."}} immediately. Do not repeat an action you already took successfully.

Respond with ONLY the JSON object for your NEXT single action. Do not combine multiple tools in one response."#
    );

    let request_body = OllamaRequest {
        model: model.to_string(),
        prompt: system_prompt,
        stream: false,
        format: "json".to_string(),
    };

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request_body)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;

    match serde_json::from_str::<AgentAction>(&response.response) {
        Ok(action) => Ok(action),
        Err(e) => {
            println!("Model returned bad JSON: {}\nRaw response: {}", e, response.response);
            Ok(AgentAction {
                tool: "done".to_string(),
                path: None,
                content: None,
                command: None,
                reasoning: Some("Stopped due to invalid model output".to_string()),
            })
        }
    }
}

// --- Section 8 ---
pub async fn run_agent(task: &str, model: &str, max_steps: u32) -> anyhow::Result<(String, u32)> {
    let client = reqwest::Client::new();
    let mut history = String::new();
    let mut final_result = String::new();

    for step in 1..=max_steps {
        println!("\n--- Step {} (model: {}) ---", step, model);
        let action = ask_model(&client, task, &history, model).await?;
        let reasoning = action.reasoning.clone().unwrap_or_else(|| "(no reasoning given)".to_string());
        println!("AI reasoning: {}", reasoning);

        match action.tool.as_str() {
            "read_file" => {
                let path = action.path.unwrap_or_default();
                let content = tools::read_file(&path).unwrap_or_else(|e| format!("Error: {}", e));
                println!("Read file: {}", path);
                history.push_str(&format!("\nRead {}: {}", path, content));
            }
            "write_file" => {
                let path = action.path.unwrap_or_default();
                let content = action.content.unwrap_or_default();
                tools::write_file(&path, &content)?;
                println!("Wrote file: {}", path);
                history.push_str(&format!(
                    "\nWrote file '{}' successfully. The task is now COMPLETE. Respond with 'done' next.",
                    path
                ));
                final_result = format!("Wrote file: {}", path);
            }
            "run_command" => {
                let cmd = action.command.unwrap_or_default();
                let output = tools::run_command(&cmd).unwrap_or_else(|e| format!("Error: {}", e));
                println!("Command output: {}", output);
                history.push_str(&format!("\nRan '{}': {}", cmd, output));
                final_result = output;
            }
            "done" => {
                final_result = format!("Task complete after {} steps.", step);
                println!("Task complete!");
                return Ok((final_result, step));
            }
            _ => {
                final_result = "Unknown tool, stopped.".to_string();
                println!("Unknown tool, stopping.");
                return Ok((final_result, step));
            }
        }
    }

    Ok((final_result, max_steps))
}
// --- END Section 8 ---

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let valid_keys = load_api_keys();
    println!("Loaded {} API key(s)", valid_keys.len());

    let app = api::create_router(valid_keys);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Forge-1 API running at http://127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}