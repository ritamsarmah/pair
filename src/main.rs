use anyhow::{Context, Result, bail};
use reqwest::header::CONTENT_TYPE;
use serde_json::{Value, json};
use std::{
    env,
    fs::read_to_string,
    io::{self, Read},
    process::{Command, exit},
};
use termimad::{MadSkin, crossterm::style::Color};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const MODEL: &str = "gemini-2.5-flash";
const USAGE: &str = "Usage: pair [OPTIONS] <COMMAND> [ARGS]

Commands:
  -c, --code <PROMPT>       Output code based on instructions
  -r, --review [<FILES>...] Review specified or modified files

Options:
  -h, --help                Show this help message and exit";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{USAGE}");
        exit(1);
    }

    let flag = &args[1];
    match flag.as_ref() {
        "-c" | "--code" => {
            let instructions = if args.len() > 2 {
                args[2..].join(" ")
            } else {
                "Finish the implementation and output the complete code".to_owned()
            };

            write_code(&instructions)?
        }
        "-r" | "--review" => {
            let code = if args.len() > 2 {
                // Review specified files
                let paths = &args[2..];
                let mut code = String::new();

                for path in paths {
                    let content = read_to_string(path)
                        .with_context(|| format!("Failed to read file '{path}'"))?;
                    code.push_str(&format!("[{path}]"));
                    code.push_str(&content);
                    code.push('\n');
                }

                code
            } else {
                // No files specified; fetch modified files in version control
                let output = Command::new("git").arg("diff").output()?.stdout;
                let diff = String::from_utf8_lossy(&output);

                if diff.is_empty() {
                    bail!("No code changes found");
                }

                diff.to_string()
            };

            review_code(&code)?
        }
        "-h" | "--help" => println!("{USAGE}"),
        _ => bail!("Unrecognized flag: {}", flag),
    }

    Ok(())
}

/// Writes code based on instructions to stdout.
fn write_code(instructions: &str) -> Result<()> {
    let prompt = "You are an expert programmer.
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

/// Reviews code for bugs and issues.
fn review_code(code: &str) -> Result<()> {
    let prompt = "You are a senior software engineer performing a professional code review.
    Analyze the code step-by-step and return only real issues. Never give compliments or stylistic opinions.

    Focus on the following categories:
    1. Bugs (logic errors, incorrect assumptions, edge cases)
    2. Performance (slow paths, unnecessary allocations, poor complexity)
    3. Maintainability (readability, clarity, duplication, structure)
    4. Security (vulnerabilities, injections, unsafe practices)

    Output:
    - Always use Markdown, with an h2 header for each section.
    - Be concise, but specific. No vague or generic comments.
    - List only real, significant issues. Don't invent issues if none are present.
    - Never wrap the Markdown output with code fence backticks.";

    let response = llm_response(prompt, &code)?;
    let skin = get_markdown_skin();
    skin.print_text(&response);

    Ok(())
}

fn llm_response(prompt: &str, contents: &str) -> Result<String> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let body = json!({
        "system_instruction": { "parts": [{ "text": prompt }] },
        "contents": [{ "parts": [{ "text": contents }] }],
        "generationConfig": {
            "temperature": 0,
            "thinkingConfig": { "thinkingBudget": 0 }
        }
    });

    let url = format!("{BASE_URL}/{MODEL}:generateContent");
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
