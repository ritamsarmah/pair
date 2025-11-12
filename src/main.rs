use anyhow::{Context, Result, bail};
use nanoserde::{DeJson, SerJson};
use std::{
    collections::HashMap,
    env,
    fs::read_to_string,
    io::{self, Read},
    process::{Command, exit},
};

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/completions";

const USAGE: &str = "Usage: pair [OPTIONS] <COMMAND> [ARGS]

Commands:
  -c, --code <PROMPT>       Output code based on instructions
  -r, --review [<FILES>...] Review specified or modified files

Options:
  -h, --help                Show this help message and exit";

/* Response */

mod openrouter {
    use nanoserde::DeJson;

    #[derive(DeJson, Debug)]
    pub struct Response {
        pub error: Option<Error>,
        pub choices: Option<Vec<Choice>>,
    }

    #[derive(DeJson, Debug)]
    pub struct Error {
        pub message: String,
    }

    #[derive(DeJson, Debug)]
    pub struct Choice {
        pub text: String,
    }
}

/* Functions */

fn main() -> Result<()> {
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

            write_code(&instructions)?
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

            review_code(&code)?
        }
        "-h" | "--help" => println!("{USAGE}"),
        _ => bail!("Unrecognized flag: {}", flag),
    }

    Ok(())
}

/// Writes code based on instructions to stdout.
fn write_code(instructions: &str) -> Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let prompt = format!(
        "Instructions: {}\nInput: {}",
        instructions.trim(),
        input.trim()
    );

    let response = llm_response("@preset/write-code", &prompt)?;
    println!("{response}");

    Ok(())
}

/// Reviews code for bugs and issues.
fn review_code(code: &str) -> Result<()> {
    let response = llm_response("@preset/review-code", code)?;
    print_markdown(&response);

    Ok(())
}

fn llm_response(preset: &str, prompt: &str) -> Result<String> {
    let api_key = env::var("OPENROUTER_API_KEY").context("No valid OpenRouter API key found")?;

    let mut body = HashMap::<String, String>::new();
    body.insert("model".into(), preset.into());
    body.insert("prompt".into(), prompt.into());

    let response = ureq::post(OPENROUTER_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .send(body.serialize_json())?;

    let text = response.into_body().read_to_string()?;
    let response = openrouter::Response::deserialize_json(&text)?;

    if let Some(error) = response.error {
        bail!(error.message);
    }

    if let Some(choices) = response.choices {
        Ok(choices[0].text.clone())
    } else {
        bail!("Invalid response format")
    }
}

fn print_markdown(markdown: &str) {
    println!("{markdown}");
}
