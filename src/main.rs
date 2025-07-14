use anyhow::Result;
use reqwest::header::CONTENT_TYPE;
use serde_json::{Value, json};
use std::{
    env,
    io::{self, Read},
    process::exit,
};

const SYSTEM_PROMPT: &str = "You are an expert pair programmer. Follow provided instructions and output minimal, idiomatic, raw code, without any Markdown formatting (no code fence backticks) or HTML tags. Keep any comments and style.";
const BASE_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

fn main() -> Result<()> {
    let mut instructions = env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();

    if instructions.is_empty() {
        instructions = "Finish the implementation and output the complete code".into();
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let contents = format!(
        "Instructions: {}\nInput: {}",
        instructions.trim(),
        input.trim()
    );

    let body = json!({
        "system_instruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
        "contents": [{ "parts": [{ "text": contents }] }],
        "generationConfig": { "thinkingConfig": { "thinkingBudget": 0 } }
    });

    let api_key = env::var("GEMINI_API_KEY")?;

    let client = reqwest::blocking::Client::new();
    let response: Value = client
        .post(BASE_URL)
        .header("x-goog-api-key", api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()?
        .json()?;

    if let Some(text) = response["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        println!("{text}");
        Ok(())
    } else {
        eprintln!("Failed to extract response text");
        exit(2);
    }
}
