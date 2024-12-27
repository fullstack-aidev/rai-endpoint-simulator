use futures_util::Stream;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc::{channel, Sender};
use log::{info, debug, error};
use rand::Rng;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct Chunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    system_fingerprint: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    delta: Delta,
    logprobs: Option<serde_json::Value>,
    finish_reason: Option<String>,
}

#[derive(Serialize)]
struct Delta {
    content: String,
}

#[derive(Serialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    prompt_tokens_details: PromptTokensDetails,
    completion_tokens_details: CompletionTokensDetails,
}

#[derive(Serialize)]
struct PromptTokensDetails {
    cached_tokens: u32,
    audio_tokens: u32,
}

#[derive(Serialize)]
struct CompletionTokensDetails {
    reasoning_tokens: u32,
    audio_tokens: u32,
    accepted_prediction_tokens: u32,
    rejected_prediction_tokens: u32,
}

fn generate_id() -> String {
    let prefix = "chatcmpl-Ai";
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    format!("{}{}", prefix, suffix)
}

fn split_into_chunks(input: &str) -> Vec<String> {
    let chunk_size = 100; // Adjust chunk size as needed
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

        if tx.send(combined_chunk.clone()).await.is_err() {
            error!("Failed to send chunk: {}", combined_chunk);
            break;
        } else {
            debug!("Sent chunk: {}", combined_chunk);
        }

        sleep(Duration::from_millis(100)).await;
    }

    let final_chunk = Chunk {
        id: generate_id(),
        object: "chat.completion.chunk".to_string(),
        created: 1735278816,
        model: "gpt-4o-2024-08-06".to_string(),
        system_fingerprint: "fp_d28bcae782".to_string(),
        choices: vec![],
        usage: Some(Usage {
            prompt_tokens: 182,
            completion_tokens: 520,
            total_tokens: 702,
            prompt_tokens_details: PromptTokensDetails { cached_tokens: 0, audio_tokens: 0 },
            completion_tokens_details: CompletionTokensDetails {
                reasoning_tokens: 0,
                audio_tokens: 0,
                accepted_prediction_tokens: 0,
                rejected_prediction_tokens: 0,
            },
        }),
    };

    let final_chunk_str = serde_json::to_string(&final_chunk).unwrap();
    let combined_final_chunk = format!("data: {}\n\n", final_chunk_str);

    if tx.send(combined_final_chunk.clone()).await.is_err() {
        error!("Failed to send final chunk: {}", combined_final_chunk);
        return;
    } else {
        debug!("Sent final chunk: {}", combined_final_chunk);
    }

    if tx.send("data: [DONE]".to_string()).await.is_err() {
        error!("Failed to send final chunk: data: [DONE]");
        return;
    } else {
        debug!("Sent final chunk: data: [DONE]");
    }
}

pub fn openai_simulator(input: &str) -> impl Stream<Item = String> {
    info!("Starting OpenAI simulator");
    let (tx, rx) = channel(10);
    let input = input.to_string();
    tokio::spawn(async move {
        generate_chunks(tx, &input).await;
    });
    ReceiverStream::new(rx)
}