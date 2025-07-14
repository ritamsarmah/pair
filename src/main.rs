use anyhow::{Result, bail};
use reqwest::header::CONTENT_TYPE;
use serde_json::{Value, json};
use std::{
    env,
    io::{self, Read},
    process::{Command, exit},
};
use termimad::{MadSkin, crossterm::style::Color};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const USAGE: &str = "Usage: pair <flag> <prompt>

Flags:
  -c, --code    Output code based on instructions
  -r, --review  Review current changes in version control
  -h, --help    Show this help message and exit";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{USAGE}");
        exit(1);
    }

    let flag = &args[1];
    match flag.as_ref() {
        "-c" | "--code" => {
            let instructions = args
                .get(2)
                .cloned()
                .unwrap_or("Finish the implementation and output the complete code".to_owned());

            write_code(&instructions)?
        }
        "-r" | "--review" => review_changes()?,
        "-h" | "--help" => println!("{USAGE}"),
        _ => bail!("Unrecognized flag: {}", flag),
    }

    Ok(())
}

/// Writes code based on instructions to stdout
fn write_code(instructions: &str) -> Result<()> {
    let prompt = "You are an expert pair programmer.
    Follow provided instructions and output minimal, idiomatic, raw code, without any Markdown formatting (no code fence backticks) or HTML tags.
    Keep any comments and style.";

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let contents = format!(
        "Instructions: {}\nInput: {}",
        instructions.trim(),
        input.trim()
    );

    let response = llm_response(prompt, &contents)?;
    println!("{response}");

    Ok(())
}

/// Reviews current changes in version control
fn review_changes() -> Result<()> {
    let output = Command::new("git").arg("diff").output()?.stdout;
    let diff = String::from_utf8_lossy(&output);

    if diff.is_empty() {
        bail!("No code changes found")
    }

    let prompt = "You are an expert code reviewer.
    Give a short, concise code review considering:
    1. Bugs
    2. Performance
    3. Maintainability
    4. Security
    Only comment on issues; never validate good changes.
    Always output Markdown, with a header for each section.
    Never include surrounding code fence backticks.";

    let response = llm_response(prompt, &diff)?;
    let skin = get_markdown_skin();
    skin.print_text(&response);

    Ok(())
}

fn llm_response(prompt: &str, contents: &str) -> Result<String> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let body = json!({
        "system_instruction": { "parts": [{ "text": prompt }] },
        "contents": [{ "parts": [{ "text": contents }] }],
        "generationConfig": { "thinkingConfig": { "thinkingBudget": 0 } }
    });

    let url = format!("{BASE_URL}/gemini-2.5-flash:generateContent");
    let client = reqwest::blocking::Client::new();
    let response: Value = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()?
        .json()?;

    let text = response["candidates"][0]["content"]["parts"][0]["text"].to_string();

    Ok(serde_json::from_str(&text)?)
}

fn get_markdown_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.set_fg(Color::Reset);
    skin.set_bg(Color::Reset);
    skin.inline_code.set_fg(Color::White);
    skin.inline_code.set_bg(Color::Black);
    skin.code_block.set_fg(Color::Reset);
    skin.code_block.set_bg(Color::Reset);
    skin
}
