mod stream;
mod response;
mod config_loader;

use std::sync::{Arc, Mutex};
use actix_web::{web, App, HttpResponse, HttpServer, middleware::Logger, ResponseError};
use tokio::sync::Semaphore;
use futures_util::StreamExt; // Import StreamExt trait
use log::{info, debug, error};
use clickhouse::{Client, Row};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::response::{select_random_response_from_db, format_response_from_db, read_random_markdown_file};
use crate::stream::openai_simulator;
use crate::config_loader::Config;
use env_logger::Builder;
use once_cell::sync::Lazy;

#[derive(Debug, Display)]
enum CustomError {
    #[display(fmt = "Failed to fetch responses")]
    FetchError,
    #[display(fmt = "Invalid source configuration")]
    InvalidSource,
}

impl ResponseError for CustomError {}

impl From<clickhouse::error::Error> for CustomError {
    fn from(_error: clickhouse::error::Error) -> Self {
        CustomError::FetchError
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

async fn fetch_responses(client: Arc<Mutex<Client>>) -> Result<Vec<ResponseSimulator>, CustomError> {
    info!("Attempting to fetch responses from the database");

    let query = "SELECT qa_id, pertanyaan, jawaban, referensi FROM response_simulator";
    debug!("Executing query: {}", query);

    let client = client.lock().unwrap();
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

#[actix_web::post("/v1/chat/completions")]
async fn chat_completions(
    client: web::Data<Arc<Mutex<Client>>>,
    semaphore: web::Data<Arc<Semaphore>>,
) -> Result<HttpResponse, CustomError> {
    let _permit = semaphore.acquire().await.map_err(|_| CustomError::FetchError)?; // Acquire a permit

    info!("Received request for chat completions");

    let random_response = match CONFIG.source.as_str() {
        "file" => read_random_markdown_file("zresponse").expect("Failed to read markdown file"),
        "database" => {
            let responses = fetch_responses(client.get_ref().clone()).await?;
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
            debug!("Sending chunk: {}", chunk);
        }
        Ok::<_, actix_web::Error>(web::Bytes::from(chunk))
    });

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(stream))
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
    info!("Starting server at http://127.0.0.1:4545");

    let client = Arc::new(Mutex::new(Client::default()
        .with_url("http://localhost:8123")
        .with_database("midai_simulator")
        .with_user(CONFIG.database.username.clone())
        .with_password(CONFIG.database.password.clone())));

    if CONFIG.source == "database" {
        // Check ClickHouse connection
        match client.lock().unwrap().query("SELECT 1").execute().await {
            Ok(_) => info!("Successfully connected to ClickHouse database"),
            Err(e) => {
                error!("Failed to connect to ClickHouse database: {}", e);
                return Err(CustomError::FetchError);
            }
        }

        // Initial query to count rows in response_simulator table
        info!("Executing initial query to count rows in response_simulator table");
        match client.lock().unwrap().query("SELECT COUNT(*) FROM response_simulator").fetch_one::<u64>().await {
            Ok(count) => info!("Number of rows in response_simulator table: {}", count),
            Err(e) => error!("Failed to count rows in response_simulator table: {}", e),
        }

        if CONFIG.tracking.enabled {
            // Initial query to fetch all records from response_simulator table
            info!("Executing initial query to fetch all records from response_simulator table");

            let mut cursor = client.lock().unwrap()
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

    let semaphore = Arc::new(Semaphore::new(500)); // Limit to 10 concurrent requests

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(semaphore.clone()))
            .service(chat_completions)
    })
        .bind("127.0.0.1:4545")
        .map_err(|_| CustomError::FetchError)? // Convert the error type
        .run()
        .await
        .map_err(|_| CustomError::FetchError) // Convert the error type
}