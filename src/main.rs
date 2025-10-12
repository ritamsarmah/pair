use anyhow::{Context, Result, bail};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};
use std::{
    env,
    fs::read_to_string,
    io::{self, Read},
    process::{Command, exit},
};
use termimad::{MadSkin, crossterm::style::Color};

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/completions";

const USAGE: &str = "Usage: pair [OPTIONS] <COMMAND> [ARGS]

Commands:
  -c, --code <PROMPT>       Output code based on instructions
  -r, --review [<FILES>...] Review specified or modified files

Options:
  -h, --help                Show this help message and exit";

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{USAGE}");
        exit(1);
    }

    let flag = &args[1];
    match flag.as_ref() {
        "-c" | "--code" => {
            let instructions = args.get(2..).map_or_else(
                || "Finish the implementation and output the complete code".to_owned(),
                |s| s.join(" "),
            );

            write_code(&instructions).await?
        }
        "-r" | "--review" => {
            let code = if args.len() > 2 {
                // Review specified files
                args[2..]
                    .iter()
                    .map(|path| {
                        read_to_string(path)
                            .with_context(|| format!("Failed to read file '{path}'"))
                            .map(|content| format!("[{path}]\n{content}\n"))
                    })
                    .collect::<Result<String>>()?
            } else {
                // No files specified. Fetch modified files in version control
                let output = Command::new("git")
                    .args(["diff", "--text"])
                    .output()?
                    .stdout;
                let diff = String::from_utf8_lossy(&output);

                if diff.is_empty() {
                    bail!("No code changes found");
                }

                diff.to_string()
            };

            review_code(&code).await?
        }
        "-h" | "--help" => println!("{USAGE}"),
        _ => bail!("Unrecognized flag: {}", flag),
    }

    Ok(())
}

/// Writes code based on instructions to stdout.
async fn write_code(instructions: &str) -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let prompt = format!(
        "Instructions: {}\nInput: {}",
        instructions.trim(),
        input.trim()
    );

    let response = llm_response("@preset/write-code", &prompt).await?;
    println!("{response}");

    Ok(())
}

/// Reviews code for bugs and issues.
async fn review_code(code: &str) -> Result<()> {
    let response = llm_response("@preset/review-code", code).await?;
    let skin = get_markdown_skin();
    skin.print_text(&response);

    Ok(())
}

async fn llm_response(preset: &str, prompt: &str) -> Result<String> {
    let api_key = env::var("OPENROUTER_API_KEY").context("No valid OpenRouter API key found")?;
    let client = reqwest::Client::new();

    let body = json!({
        "model": preset,
        "prompt": prompt
    });

    let request = client
        .post(OPENROUTER_URL)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .json(&body);

    let response: Value = request.send().await?.json().await?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .map_or("Unknown OpenRouter error".to_owned(), |m| m.to_string());
        bail!(message);
    }

    let output = response["choices"][0]["text"]
        .as_str()
        .context("Invalid response format")?;

    Ok(output.to_string())
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
