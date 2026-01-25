package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"strings"

	"golang.org/x/term"
)

const OpenRouterURL = "https://openrouter.ai/api/v1/completions"

const Usage = `Usage: pair <FLAG> [ARGS]

Flags:
  -a, --answer <PROMPT>     Answer a general query
  -c, --code <PROMPT>       Output code based on instructions
  -r, --review [<FILES>...] Review specified or modified files
  -h, --help                Show this help message and exit`

/* Main */

func main() {
	args := os.Args
	if len(args) < 2 {
		fmt.Println(Usage)
		return
	}

	flag := args[1]
	input := args[2:]

	var output string
	var err error

	switch flag {
	case "-a", "--answer":
		output, err = answer(joined(input))
	case "-c", "--code":
		output, err = writeCode(joined(input))
	case "-r", "--review":
		output, err = reviewCode(input)
	case "-h", "--help":
		output = Usage
	default:
		err = fmt.Errorf("unrecognized flag: %s", flag)
	}

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}

	fmt.Println(output)
}

func answer(query string) (string, error) {
	if query == "" {
		return "", errors.New("no query provided")
	}
	return llmResponse("@preset/answer-query", query)
}

func writeCode(instructions string) (string, error) {
	var input string

	if !term.IsTerminal(int(os.Stdin.Fd())) {
		data, err := io.ReadAll(os.Stdin)
		if err != nil {
			return "", err
		}
		input = string(data)
	}

	instructions = strings.TrimSpace(instructions)

	var prompt string
	switch {
	case input != "" && instructions != "":
		prompt = fmt.Sprintf("Instructions:\n%s\n\nInput:\n%s", instructions, input)
	case input == "" && instructions != "":
		prompt = instructions
	case input != "" && instructions == "":
		prompt = fmt.Sprintf("Finish the implementation and output the complete code:\n%s", input)
	default:
		return "", errors.New("no input code or instructions provided")
	}

	return llmResponse("@preset/write-code", prompt)
}

func reviewCode(paths []string) (string, error) {
	var code string

	if len(paths) == 0 {
		fmt.Printf("Reviewing changed files under version control\n\n")
		cmd := exec.Command("git", "diff", "--text")
		out, err := cmd.Output()
		if err != nil {
			return "", errors.New("failed to fetch git diff")
		}
		code = string(out)
	} else {
		fmt.Printf("Reviewing %d specified file(s)\n\n", len(paths))
		var b strings.Builder
		for _, path := range paths {
			data, err := os.ReadFile(path)
			if err != nil {
				return "", fmt.Errorf("failed to read file: %s", path)
			}
			fmt.Fprintf(&b, "[%s]\n%s\n\n", path, data)
		}
		code = b.String()
	}

	if strings.TrimSpace(code) == "" {
		return "", errors.New("no code changes found")
	}

	return llmResponse("@preset/review-code", code)
}

func llmResponse(preset, prompt string) (string, error) {
	apiKey := os.Getenv("OPENROUTER_API_KEY")
	if apiKey == "" {
		return "", errors.New("no valid OpenRouter API key found")
	}

	body := map[string]string{"model": preset, "prompt": prompt}
	data, err := json.Marshal(body)
	if err != nil {
		return "", err
	}

	req, err := http.NewRequest("POST", OpenRouterURL, bytes.NewReader(data))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+apiKey)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	var result struct {
		Error   *struct{ Message string } `json:"error,omitempty"`
		Choices []struct{ Text string }   `json:"choices,omitempty"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", err
	}

	if result.Error != nil {
		return "", errors.New(result.Error.Message)
	}

	if len(result.Choices) == 0 {
		return "", errors.New("invalid response format")
	}

	return result.Choices[0].Text, nil
}

func joined(input []string) string {
	return strings.TrimSpace(strings.Join(input, " "))
}
