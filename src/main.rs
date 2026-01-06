use anyhow::{Context, Result, bail};
use nanoserde::{DeJson, SerJson};
use std::{
    collections::HashMap,
    env,
    fs::read_to_string,
    io::{self, IsTerminal, Read},
    process::Command,
};

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/completions";

const USAGE: &str = "Usage: pair <COMMAND> [ARGS]

Commands:
  -a, --ask <PROMPT>        Answer a general query
  -c, --code <PROMPT>       Output code based on instructions
  -r, --review [<FILES>...] Review specified or modified files
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
        return Ok(());
    }

    let flag = &args[1];
    let input = args.get(2..).unwrap_or(&[]);

    let output = match flag.as_ref() {
        "-a" | "--ask" => ask(&input.join(" ").trim())?,
        "-c" | "--code" => write_code(&input.join(" ").trim())?,
        "-r" | "--review" => review_code(input)?,
        "-h" | "--help" => USAGE.into(),
        _ => bail!("Unrecognized flag: {}", flag),
    };

    println!("{output}");

    Ok(())
}

/// Generate response for query.
fn ask(query: &str) -> Result<String> {
    if query.is_empty() {
        bail!("No query provided")
    }

    llm_response("@preset/ask-query", query)
}

/// Write code based on instructions.
fn write_code(instructions: &str) -> Result<String> {
    // Read input code from stdin, otherwise using instructions to generate code
    let mut input = String::new();
    if !io::stdin().is_terminal() {
        io::stdin().read_to_string(&mut input)?;
    }

    let input = input.trim();
    let instructions = instructions.trim();

    let prompt = match (input.is_empty(), instructions.is_empty()) {
        (true, true) => bail!("No input code or instructions provided"),
        (true, false) => instructions.into(),
        (false, true) => {
            format!("Finish the implementation and output the complete code:\n{input}")
        }
        (false, false) => format!("Instructions:\n{instructions}\n\nInput:\n{input}",),
    };

    llm_response("@preset/write-code", &prompt)
}

/// Review code for bugs and issues.
fn review_code(paths: &[String]) -> Result<String> {
    let code = if paths.is_empty() {
        println!("Reviewing changed files under version control\n");
        let output = Command::new("git").args(["diff", "--text"]).output()?;

        if !output.status.success() {
            bail!("Failed to fetch git diff")
        }

        String::from_utf8_lossy(&output.stdout).into()
    } else {
        println!("Reviewing {} specified file(s)\n", paths.len());

        paths
            .iter()
            .map(|path| {
                read_to_string(path)
                    .with_context(|| format!("Failed to read file: {}", path))
                    .map(|content| format!("[{path}]\n{content}\n"))
            })
            .collect::<Result<Vec<String>>>()?
            .join("\n")
    };

    if code.trim().is_empty() {
        bail!("No code changes found");
    }

    llm_response("@preset/review-code", &code)
}

/// Retrieve LLM response using OpenRouter API.
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
