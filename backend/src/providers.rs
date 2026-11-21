//! Server-only provider adapters. Never move provider credentials into the WASM client.
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Groq,
    Gemini,
}

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider is not configured")]
    MissingConfig,
    #[error("provider rejected the request ({0})")]
    Upstream(StatusCode),
    #[error("provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("provider returned an invalid response")]
    InvalidResponse,
}

pub async fn complete(
    client: &Client,
    config: &ProviderConfig,
    messages: &[ChatMessage],
    system: &str,
) -> Result<String, ProviderError> {
    if config.api_key.trim().is_empty() || config.model.trim().is_empty() {
        return Err(ProviderError::MissingConfig);
    }

    match config.provider {
        Provider::Groq => groq(client, config, messages, system).await,
        Provider::Gemini => gemini(client, config, messages, system).await,
    }
}

async fn groq(
    client: &Client,
    config: &ProviderConfig,
    messages: &[ChatMessage],
    system: &str,
) -> Result<String, ProviderError> {
    let mut all = Vec::with_capacity(messages.len() + 1);
    if !system.is_empty() {
        all.push(json!({"role":"system","content":system}));
    }
    for message in messages {
        let role = match message.role.as_str() {
            "assistant" => "assistant",
            _ => "user",
        };
        all.push(json!({"role":role,"content":message.content}));
    }

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(&config.api_key)
        .json(&json!({"model":config.model,"messages":all,"stream":false}))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ProviderError::Upstream(response.status()));
    }
    let body: Value = response.json().await?;
    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(ProviderError::InvalidResponse)
}

async fn gemini(
    client: &Client,
    config: &ProviderConfig,
    messages: &[ChatMessage],
    system: &str,
) -> Result<String, ProviderError> {
    let contents: Vec<Value> = messages
        .iter()
        .map(|message| json!({
            "role": if message.role == "assistant" { "model" } else { "user" },
            "parts": [{"text": message.content}]
        }))
        .collect();

    let mut request = json!({"contents":contents});
    if !system.is_empty() {
        request["systemInstruction"] = json!({"parts":[{"text":system}]});
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        config.model
    );
    let response = client
        .post(url)
        .header("x-goog-api-key", &config.api_key)
        .json(&request)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ProviderError::Upstream(response.status()));
    }
    let body: Value = response.json().await?;
    body.pointer("/candidates/0/content/parts/0/text")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(ProviderError::InvalidResponse)
}
