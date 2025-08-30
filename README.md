# pair

An AI pair programmer for terminal-based workflows.

## Features

**Code Completion**

```sh
echo 'print("hello' | pair --code
```

**Code Completion with Custom Prompt**

```sh
echo 'print("hello world")' | pair --code rewrite this code in Rust
```

**Review Local Git Changes**

```sh
pair --review
```

**Review Specific File(s)**

```sh
pair --review src/main.rs src/utils.rs
```

## Getting Started

1. Clone this repository.
2. Build the project with `cargo build --release`.
3. Install the binary to a location on your system's `PATH`, e.g., `/usr/local/bin`.
4. Set the "OPENROUTER_API_KEY" environment variable.
5. Run the application with `pair`.
