# Ollama Chat Interface

A simple, browser-based chat interface for [Ollama](https://ollama.ai) built with Rust and modern web technologies.

![Ollama Chat Interface Screenshot](https://i.imgur.com/placeholder.png)

## Features

- Clean, responsive web UI for chatting with your local Ollama models
- Automatic detection of available models
- Support for file uploads to provide context to your prompts
- Streaming responses for real-time interaction
- Markdown rendering with syntax highlighting
- Automatic Ollama startup if not running

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Ollama](https://ollama.ai/download) installed on your system

## Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/ollama-app-rust.git
cd ollama-app-rust
```

2. Build the application:

```bash
cargo build --release
```

## Usage

1. Run the application:

```bash
cargo run --release
```

2. The application should automatically open your default browser to `http://localhost:3000`

3. If Ollama is not running, you can start it directly from the interface.

4. Select a model from the dropdown and start chatting!

## Configuration

The application can be configured through environment variables:

- `OLLAMA_URL`: URL for the Ollama API (default: http://127.0.0.1:11434)
- `PORT`: Port for the web interface (default: 3000)
- `RUST_LOG`: Log level (default: info)

Example:

```bash
OLLAMA_URL=http://192.168.1.100:11434 PORT=8080 RUST_LOG=debug cargo run
```

## Building for Production

```bash
cargo build --release
```

The compiled binary will be available at `./target/release/ollama-interface`

## TODO

- [ ] Add conversation context
- [ ] Add conversation history storage/persistence
- [ ] Add support for conversation management (saving, loading, deleting)
- [ ] Add dark mode toggle
- [ ] Break code into multiple files


## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [Ollama](https://ollama.ai) for making local LLMs accessible
- [Axum](https://github.com/tokio-rs/axum) for the Rust web framework
- [Tokio](https://tokio.rs/) for the async runtime
