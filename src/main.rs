mod stream;
mod response;
mod config_loader;

use std::sync::Arc;
use actix_web::{web, App, HttpResponse, HttpServer, middleware::Logger, ResponseError};
use tokio::sync::Semaphore;
use futures_util::StreamExt;
use log::{info, debug, error, warn};
use clickhouse::{Client, Row};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use crate::response::{select_random_response_from_db, format_response_from_db, read_random_markdown_file_async};
use chrono;
use crate::stream::{openai_simulator, Chunk, generate_id, PromptTokensDetails, Usage, CompletionTokensDetails};
use crate::config_loader::Config;
use env_logger::Builder;
use once_cell::sync::Lazy;

#[derive(Debug, Display)]
enum CustomError {
    #[display(fmt = "Failed to fetch responses")]
    FetchError,
    #[display(fmt = "Invalid source configuration")]
    InvalidSource,
    #[display(fmt = "Failed to bind server: {}", _0)]
    BindError(String),
    #[display(fmt = "Redis error: {}", _0)]
    RedisError(String),
}

impl ResponseError for CustomError {}

impl From<clickhouse::error::Error> for CustomError {
    fn from(_error: clickhouse::error::Error) -> Self {
        CustomError::FetchError
    }
}

impl From<std::io::Error> for CustomError {
    fn from(error: std::io::Error) -> Self {
        CustomError::BindError(error.to_string())
    }
}

impl From<redis::RedisError> for CustomError {
    fn from(error: redis::RedisError) -> Self {
        CustomError::RedisError(error.to_string())
    }
}

#[derive(Row, Deserialize, Serialize, Debug, Clone)]
struct ResponseSimulator {
    #[serde(default, with = "clickhouse::serde::uuid::option")]
    qa_id: Option<Uuid>,
    pertanyaan: String,
    jawaban: String,
    referensi: String,
}

static CONFIG: Lazy<Config> = Lazy::new(|| Config::load());

/// Application state shared across workers
struct AppState {
    db_client: Client,
    redis: ConnectionManager,
}

impl AppState {
    fn new(db_client: Client, redis: ConnectionManager) -> Self {
        Self {
            db_client,
            redis,
        }
    }
}

/// Redis key helpers
fn redis_key_db_responses(prefix: &str) -> String {
    format!("{}:db_responses", prefix)
}

fn redis_key_file_content(prefix: &str, filename: &str) -> String {
    format!("{}:file:{}", prefix, filename)
}

fn redis_key_file_list(prefix: &str) -> String {
    format!("{}:file_list", prefix)
}

/// Fetch responses from database
async fn fetch_responses_from_db(client: &Client) -> Result<Vec<ResponseSimulator>, CustomError> {
    info!("Fetching responses from the database");

    let query = "SELECT qa_id, pertanyaan, jawaban, referensi FROM response_simulator";
    debug!("Executing query: {}", query);

    let mut cursor = client.query(query).fetch::<ResponseSimulator>()?;

    let mut records = Vec::new();
    while let Ok(Some(row)) = cursor.next().await {
        records.push(row);
    }

    info!("Fetched {} records from response_simulator table", records.len());
    if CONFIG.tracking.enabled {
        for record in &records {
            debug!("{:?}", record);
        }
    }

    Ok(records)
}

/// Get cached responses from Redis, or fetch from database if cache miss/expired
async fn get_cached_db_responses(state: &AppState) -> Result<Vec<ResponseSimulator>, CustomError> {
    let mut redis = state.redis.clone();
    let key = redis_key_db_responses(&CONFIG.redis.prefix);

    // Try to get from Redis cache
    let cached: Option<String> = redis.get(&key).await.unwrap_or(None);

    if let Some(cached_json) = cached {
        match serde_json::from_str::<Vec<ResponseSimulator>>(&cached_json) {
            Ok(responses) => {
                debug!("Cache hit: returning {} cached responses from Redis", responses.len());
                return Ok(responses);
            }
            Err(e) => {
                warn!("Failed to deserialize cached responses: {}", e);
                // Continue to fetch fresh data
            }
        }
    }

    // Cache miss or error, fetch from database
    info!("Cache miss, fetching from database");
    let responses = fetch_responses_from_db(&state.db_client).await?;

    // Store in Redis with TTL
    if !responses.is_empty() {
        match serde_json::to_string(&responses) {
            Ok(json) => {
                let ttl = CONFIG.cache_ttl as i64;
                if let Err(e) = redis.set_ex::<_, _, ()>(&key, &json, ttl as u64).await {
                    warn!("Failed to cache responses in Redis: {}", e);
                } else {
                    debug!("Cached {} responses in Redis with TTL {}s", responses.len(), ttl);
                }
            }
            Err(e) => {
                warn!("Failed to serialize responses for caching: {}", e);
            }
        }
    }

    Ok(responses)
}

/// Get cached file content from Redis, or read from disk if cache miss
async fn get_cached_file_response(state: &AppState, folder_path: &str) -> Result<String, CustomError> {
    let mut redis = state.redis.clone();

    // Get list of files from cache or scan directory
    let file_list_key = redis_key_file_list(&CONFIG.redis.prefix);
    let cached_list: Option<String> = redis.get(&file_list_key).await.unwrap_or(None);

    let files: Vec<String> = if let Some(list_json) = cached_list {
        serde_json::from_str(&list_json).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    // If no cached file list, scan directory and cache it
    let files = if files.is_empty() {
        let folder = folder_path.to_string();
        let scanned_files = tokio::task::spawn_blocking(move || {
            std::fs::read_dir(&folder)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "md"))
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|_| Vec::new())
        })
        .await
        .map_err(|_e| CustomError::FetchError)?;

        // Cache file list with longer TTL (10 minutes)
        if !scanned_files.is_empty() {
            if let Ok(json) = serde_json::to_string(&scanned_files) {
                let _ = redis.set_ex::<_, _, ()>(&file_list_key, &json, 600u64).await;
            }
        }

        scanned_files
    } else {
        files
    };

    if files.is_empty() {
        error!("No markdown files found in {}", folder_path);
        return Err(CustomError::FetchError);
    }

    // Select random file
    let random_idx = rand::random::<usize>() % files.len();
    let selected_file = &files[random_idx];
    let file_key = redis_key_file_content(&CONFIG.redis.prefix, selected_file);

    // Try to get file content from Redis
    let cached_content: Option<String> = redis.get(&file_key).await.unwrap_or(None);

    if let Some(content) = cached_content {
        debug!("Cache hit: returning cached content for file {}", selected_file);
        return Ok(content);
    }

    // Cache miss, read from disk
    let file_path = format!("{}/{}", folder_path, selected_file);
    info!("Cache miss, reading file from disk: {}", file_path);

    let content = read_random_markdown_file_async(folder_path).await.map_err(|e| {
        error!("Failed to read markdown file: {}", e);
        CustomError::FetchError
    })?;

    // Cache file content with TTL
    let ttl = CONFIG.cache_ttl as u64;
    if let Err(e) = redis.set_ex::<_, _, ()>(&file_key, &content, ttl).await {
        warn!("Failed to cache file content in Redis: {}", e);
    } else {
        debug!("Cached file content in Redis with TTL {}s", ttl);
    }

    Ok(content)
}

#[actix_web::get("/health")]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "rai-endpoint-simulator",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[actix_web::post("/test_completion")]
async fn test_completion() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "id": "chatcmpl-AjoahzpVUCsJmOQZRKZUze7qBjEjn",
        "object": "chat.completion",
        "created": 1735482595,
        "model": "gpt-4o-2024-08-06",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "============>>  Selamat! Aplikasi anda telah sukses terhubung ke OpenAI Simulator. <============="
                },
                "logprobs": null,
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 57,
            "completion_tokens": 92,
            "total_tokens": 149
        }
    }))
}

#[actix_web::post("/v1/chat/completions")]
async fn chat_completions(
    state: web::Data<Arc<AppState>>,
    semaphore: web::Data<Arc<Semaphore>>,
) -> Result<HttpResponse, CustomError> {
    let _permit = semaphore.acquire().await.map_err(|_| CustomError::FetchError)?;

    info!("Received request for chat completions");

    let random_response = match CONFIG.source.as_str() {
        "file" => {
            get_cached_file_response(&state, "zresponse").await?
        },
        "database" => {
            let responses = get_cached_db_responses(&state).await?;
            if responses.is_empty() {
                error!("No responses available");
                return Err(CustomError::FetchError);
            }
            let response = select_random_response_from_db(&responses);
            debug!("Selected Response: {:?}", response);
            format_response_from_db(response)
        },
        _ => {
            error!("Invalid source configuration");
            return Err(CustomError::InvalidSource);
        }
    };

    let stream = openai_simulator(&random_response);

    let stream = stream.map(|chunk| {
        if CONFIG.tracking.enabled {
            //debug!("Sending chunk: {}", chunk);
        }
        Ok::<_, actix_web::Error>(web::Bytes::from(chunk))
    });

    let final_stream = stream.chain(futures_util::stream::once(async {
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

        let final_chunk_str = match serde_json::to_string(&final_chunk) {
            Ok(str) => str,
            Err(e) => {
                error!("Failed to serialize final chunk: {}", e);
                return Ok::<_, actix_web::Error>(web::Bytes::from("data: [ERROR]\n\n"));
            }
        };

        let combined_final_chunk = format!("data: {}\n\n", final_chunk_str);

        if CONFIG.tracking.enabled {
            info!("Sending final chunk: {}", combined_final_chunk);
        }

        Ok::<_, actix_web::Error>(web::Bytes::from(combined_final_chunk))
    }));

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(final_stream))
}

#[actix_web::main]
async fn main() -> Result<(), CustomError> {
    let log_level = match CONFIG.log_level.as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };

    Builder::new()
        .filter(None, log_level)
        .init();

    info!("Starting server at http://{}:{}", CONFIG.binding.host, CONFIG.binding.port);
    info!("Configuration: workers={}, semaphore_limit={}, cache_ttl={}s",
          CONFIG.workers, CONFIG.semaphore_limit, CONFIG.cache_ttl);

    // Initialize Redis connection
    info!("Connecting to Redis at {}", CONFIG.redis.url);
    let redis_client = redis::Client::open(CONFIG.redis.url.as_str())
        .map_err(|e| CustomError::RedisError(format!("Failed to create Redis client: {}", e)))?;

    let redis_conn = ConnectionManager::new(redis_client)
        .await
        .map_err(|e| CustomError::RedisError(format!("Failed to connect to Redis: {}", e)))?;

    info!("Successfully connected to Redis");

    // Initialize ClickHouse client
    let db_client = Client::default()
        .with_url(&CONFIG.database.url)
        .with_database("midai_simulator")
        .with_user(CONFIG.database.username.clone())
        .with_password(CONFIG.database.password.clone());

    if CONFIG.source == "database" {
        match db_client.query("SELECT 1").execute().await {
            Ok(_) => info!("Successfully connected to ClickHouse database"),
            Err(e) => {
                error!("Failed to connect to ClickHouse database: {}", e);
                return Err(CustomError::FetchError);
            }
        }

        info!("Executing initial query to count rows in response_simulator table");
        match db_client.query("SELECT COUNT(*) FROM response_simulator").fetch_one::<u64>().await {
            Ok(count) => info!("Number of rows in response_simulator table: {}", count),
            Err(e) => error!("Failed to count rows in response_simulator table: {}", e),
        }

        if CONFIG.tracking.enabled {
            info!("Executing initial query to fetch all records from response_simulator table");

            let mut cursor = db_client
                .query("SELECT qa_id, pertanyaan, jawaban, referensi FROM response_simulator")
                .fetch::<ResponseSimulator>()?;

            let mut records = Vec::new();
            while let Ok(Some(row)) = cursor.next().await {
                records.push(row);
            }

            debug!("Fetched {} records from response_simulator table", records.len());
            for record in records {
                debug!("{:?}", record);
            }
        }
    }

    // Create shared application state
    let app_state = Arc::new(AppState::new(db_client, redis_conn));
    let semaphore = Arc::new(Semaphore::new(CONFIG.semaphore_limit));

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(semaphore.clone()))
            .service(health_check)
            .service(chat_completions)
            .service(test_completion)
    })
        .workers(CONFIG.workers)
        .bind(format!("{}:{}", CONFIG.binding.host, CONFIG.binding.port))?
        .run()
        .await
        .map_err(|e| CustomError::BindError(e.to_string()))
}
