mod response;
mod stream;
mod config_loader;

use std::time::Duration;
use actix_web::{web, App, HttpResponse, HttpServer, middleware::Logger, ResponseError};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use clickhouse::{Client, Row};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::rt::TokioExecutor;
use futures_util::StreamExt;
use log::{info, debug, error};
use once_cell::sync::Lazy;
use env_logger::Builder;
use uuid::Uuid;
use crate::response::{select_random_response, select_random_response_from_db, format_response_from_db};
use crate::stream::openai_simulator;
use crate::config_loader::Config;

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
    #[serde(with = "clickhouse::serde::uuid")]
    qa_id: Uuid,
    pertanyaan: String,
    jawaban: String,
    referensi: String,
}

static CONFIG: Lazy<Config> = Lazy::new(|| Config::load());

static CLIENT: Lazy<Client> = Lazy::new(|| {
    debug!("Initializing ClickHouse client");
    debug!("Username: {}", CONFIG.database.username);
    debug!("Password: {}", CONFIG.database.password);

    let connector = HttpConnector::new();
    let hyper_client = HyperClient::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_millis(2_500))
        .pool_max_idle_per_host(4)
        .build(connector);

    Client::with_http_client(hyper_client)
        .with_url("http://localhost:8123")
        .with_database("midai_simulator")
        .with_user(CONFIG.database.username.clone())
        .with_password(CONFIG.database.password.clone())
});

async fn fetch_responses() -> Result<Vec<ResponseSimulator>, CustomError> {
    info!("Attempting to fetch responses from the database");

    let query = "SELECT qa_id, pertanyaan, jawaban, referensi FROM response_simulator";
    debug!("Executing query: {}", query);

    let mut cursor = CLIENT.query(query).fetch::<ResponseSimulator>()?;

    let mut records = Vec::new();
    while let Ok(Some(row)) = cursor.next().await {
        records.push(row);
    }

    info!("Fetched {} records from response_simulator table", records.len());
    if CONFIG.tracking.enabled == true {
        for record in &records {
            debug!("{:?}", record);
        }
    }

    Ok(records)
}

static FILE_CONTENT: Lazy<String> = Lazy::new(|| {
    info!("Reading file content at startup");
    response::read_file_content("response.json").expect("Failed to read file content")
});

#[actix_web::post("/v1/chat/completions")]
async fn chat_completions() -> Result<HttpResponse, CustomError> {
    info!("Received request for chat completions");

    let responses = match CONFIG.source.as_str() {
        "file" => serde_json::from_str::<Vec<ResponseSimulator>>(&FILE_CONTENT)
            .expect("Failed to parse JSON"),
        "database" => fetch_responses().await?,
        _ => {
            error!("Invalid source configuration");
            return Err(CustomError::InvalidSource);
        }
    };
    if CONFIG.tracking.enabled == true {
        debug!("All response record in DB : {:?}", responses);
    }
    if responses.is_empty() {
        error!("No responses available");
        return Err(CustomError::FetchError);
    }

    let random_response = if CONFIG.source == "file" {
        select_random_response(&FILE_CONTENT)
    } else {
        let response = select_random_response_from_db(&responses);
        debug!("Selected Response: {:?}", response);
        format_response_from_db(response)
    };

    let stream = openai_simulator(&random_response);

    let stream = stream.map(|chunk| {
        if CONFIG.tracking.enabled == true {
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
    Builder::new()
        .filter(None, log::LevelFilter::Debug)
        .init();
    info!("Starting server at http://127.0.0.1:4545");

    if CONFIG.source == "database" {
        // Check ClickHouse connection
        match CLIENT.query("SELECT 1").execute().await {
            Ok(_) => info!("Successfully connected to ClickHouse database"),
            Err(e) => {
                error!("Failed to connect to ClickHouse database: {}", e);
                return Err(CustomError::FetchError);
            }
        }

        // Initial query to count rows in response_simulator table
        info!("Executing initial query to count rows in response_simulator table");
        match CLIENT.query("SELECT COUNT(*) FROM response_simulator").fetch_one::<u64>().await {
            Ok(count) => info!("Number of rows in response_simulator table: {}", count),
            Err(e) => error!("Failed to count rows in response_simulator table: {}", e),
        }

        if CONFIG.tracking.enabled == true {
            // Initial query to fetch all records from response_simulator table
            info!("Executing initial query to fetch all records from response_simulator table");

            let mut cursor = CLIENT
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

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(chat_completions)
    })
        .bind("127.0.0.1:4545")
        .map_err(|_e| CustomError::FetchError)? // Prefix unused variable with an underscore
        .run()
        .await
        .map_err(|_e| CustomError::FetchError) // Prefix unused variable with an underscore
}