// src/stream.rs

use futures_util::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc::{channel, Sender};
use log::{info, debug, error};
use rand::Rng;
use serde::Serialize;
use crate::CONFIG;

#[derive(Serialize)]
pub struct Chunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub system_fingerprint: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Serialize)]
pub struct Choice {
    pub index: u32,
    pub delta: Delta,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: Option<String>,
}

#[derive(Serialize)]
pub struct Delta {
    pub content: String,
}

#[derive(Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: PromptTokensDetails,
    pub completion_tokens_details: CompletionTokensDetails,
}

#[derive(Serialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: u32,
    pub audio_tokens: u32,
}

#[derive(Serialize)]
pub struct CompletionTokensDetails {
    pub reasoning_tokens: u32,
    pub audio_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

pub fn generate_id() -> String {
    let prefix = "chatcmpl-Ai";
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    format!("{}{}", prefix, suffix)
}

fn split_into_chunks(input: &str) -> Vec<String> {
    let chunk_size = 10; // Adjust chunk size as needed
    input
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect()
}

async fn generate_chunks(tx: Sender<String>, input: &str) {
    info!("Generating chunks for input");
    let content_chunks = split_into_chunks(input);

    for content in content_chunks.iter() {
        let chunk = Chunk {
            id: generate_id(),
            object: "chat.completion.chunk".to_string(),
            created: 1735278816,
            model: "gpt-4o-2024-08-06".to_string(),
            system_fingerprint: "fp_d28bcae782".to_string(),
            choices: vec![Choice {
                index: 0,
                delta: Delta { content: content.clone() },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        };

        let chunk_str = serde_json::to_string(&chunk).unwrap();
        let combined_chunk = format!("data: {}\n\n", chunk_str);

        if let Err(e) = tx.send(combined_chunk.clone()).await {
            error!("Failed to send chunk: {}. Error: {}", combined_chunk, e);
        } else {
            debug!("Sent chunk: {}", combined_chunk);
        }
    }

    // Remove the final chunk sending from here
}

pub fn openai_simulator(input: &str) -> impl Stream<Item = String> {
    //info!("Starting OpenAI simulator");

    // Use async channel with capacity 10000
    let (tx, rx) = channel(CONFIG.channel_capacity);
    let input = input.to_string();

    tokio::spawn(async move {
        generate_chunks(tx, &input).await;
    });

    ReceiverStream::new(rx)
}