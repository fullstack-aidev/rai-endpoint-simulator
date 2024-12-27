use std::fs::File;
use std::io::{self, Read};
use rand::Rng;
use serde_json::Value;
use log::{info};

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
    let responses = json["response_chat_completion"].as_array().expect("Expected an array of responses");
    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..responses.len());
    let response = &responses[random_index];

    let question = response["Pertanyaan"].as_str().expect("Expected a string for Pertanyaan");
    let answer = response["Jawaban"].as_str().expect("Expected a string for Jawaban");

    let mut formatted_response = format!("**Pertanyaan:**\n{}\n\n**Jawaban:**\n{}", question, answer);

    if let Some(references) = response.get("Referensi") {
        let references_list = references.as_array().expect("Expected an array for Referensi");
        let references_str = references_list.iter()
            .map(|r| format!("- {}", r.as_str().expect("Expected a string in Referensi")))
            .collect::<Vec<String>>()
            .join("\n");
        formatted_response.push_str(&format!("\n\n**Referensi:**\n{}", references_str));
    }

    formatted_response.replace("\\n", "\n")
}