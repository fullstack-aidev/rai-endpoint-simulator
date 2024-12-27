use std::fs::File;
use std::io::{self, Read};
use rand::Rng;
use serde_json::Value;
use log::{info};
use rand::seq::SliceRandom;
use crate::ResponseSimulator;

pub fn read_file_content(file_path: &str) -> io::Result<String> {
    info!("Reading file content from {}", file_path);
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn select_random_response(content: &str) -> String {
    info!("Selecting random response from content");
    let json: Value = serde_json::from_str(content).expect("Invalid JSON format");
    let responses = json.as_array().expect("Expected an array of responses");
    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..responses.len());
    let response = &responses[random_index];

    let question = response["pertanyaan"].as_str().expect("Expected a string for pertanyaan");
    let answer = response["jawaban"].as_str().expect("Expected a string for jawaban");

    let mut formatted_response = format!("**Pertanyaan:**\n{}\n\n**Jawaban:**\n{}", question, answer);

    if let Some(references) = response.get("referensi") {
        let references_str = references.as_str().expect("Expected a string for referensi");
        formatted_response.push_str(&format!("\n\n**Referensi:**\n{}", references_str));
    }

    formatted_response.replace("\\n", "\n")
}

pub fn select_random_response_from_db(responses: &[ResponseSimulator]) -> &ResponseSimulator {
    info!("Selecting random response from database");
    let mut rng = rand::thread_rng();
    responses.choose(&mut rng).expect("No responses available")
}