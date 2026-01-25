# pair

An AI pair programmer for terminal-based workflows.

## Features

**Question Answering**

```sh
pair --answer 'Why is the sky blue'
```

**Code Generation**

```sh
pair --code "Write fibonacci in Go"
```

**Code Generation with Input Code**

```sh
echo 'print("hello world")' | pair --code 'rewrite this code in Go'
```

**Code Completion**

```sh
echo 'print("hello' | pair --code
```

**Review Local Git Changes**

```sh
pair --review
```

**Review Specific File(s)**

```sh
pair --review src/main.go src/utils.go
```

## Getting Started

1. Clone this repository.
2. Build the project with `go build .`.
3. Install the binary to a location on your system's `PATH`, e.g., `/usr/local/bin`.
4. Set the "OPENROUTER_API_KEY" environment variable.
5. Run the application with `pair`.
