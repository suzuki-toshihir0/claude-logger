# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Claude Logger

Real-time monitoring tool for Claude Code's JSONL conversation logs with webhook integration support.

## Build & Development Commands

```bash
# Build debug version
cargo build

# Build release version  
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- watch --latest

# Run clippy lints
cargo clippy

# Format code
cargo fmt

# Check compilation without building
cargo check

# Publish to crates.io
cargo publish
```

## High-Level Architecture

### Data Flow
```
JSONL File → LogWatcher → LogParser → LogFormatter → stdout/webhook
                ↑            ↓             ↓
            inotify     parse_file   format_message
```

### Core Components

**LogWatcher** (`src/watcher.rs`)
- Monitors `~/.claude/projects/` directory using inotify
- Manages file watching, project discovery, and event handling
- Filters messages by startup time when `include_existing=false` (default)
- Coordinates between parser, formatter, and webhook sender

**LogParser** (`src/parser.rs`)
- Parses JSONL files into structured `LogMessage` objects
- Extracts role, content, timestamp from Claude Code's log format
- Handles tool usage and thinking blocks
- Preserves raw content for detailed tool display modes

**LogFormatter** (`src/formatter.rs`)
- Formats messages for terminal output
- Supports three tool display modes: none, simple, detailed
- Extracts tool information from raw message content
- Handles timestamp formatting and role indicators

**WebhookSender** (`src/webhook.rs`)
- Sends formatted messages to external webhooks
- Supports Generic JSON and Slack formats
- Uses reqwest for async HTTP requests

### Key Design Decisions

1. **Timestamp-based filtering**: Messages are filtered by `startup_time` to prevent duplicate output when files are modified

2. **Tool display modes**: Complex tool usage can be hidden (none), simplified (🔧 Bash), or detailed (with parameters)

3. **Include existing flag**: By default (`--include-existing=false`), historical messages are skipped to prevent webhook spam

4. **Async architecture**: Uses Tokio for concurrent file watching and webhook sending

## Important CLI Options

```bash
# Watch latest project (default: skip existing messages)
claude-logger watch --latest

# Include historical messages
claude-logger watch --latest --include-existing

# Configure tool display
claude-logger watch --latest --tool-display detailed

# Enable webhook
claude-logger watch --latest --webhook-url https://hooks.slack.com/... --webhook-format slack
```

## Message Types & Formatting

The formatter handles several message types:
- User messages: `👤 User: ...`
- Assistant messages: `🤖 Claude: ...`
- Tool usage: `🔧 ToolName` or detailed parameters
- Thinking blocks: `💭 Thinking...`

## Testing Approach

```bash
# Run unit tests
cargo test

# Test specific module
cargo test webhook

# Test with pattern matching
cargo test format_slack

# Manual testing with real Claude session
# Terminal 1: claude-logger watch --latest
# Terminal 2: claude --project /path/to/project
```

## Common Issues & Solutions

1. **Duplicate message output**: Resolved by timestamp filtering in `process_jsonl_file`
2. **Webhook spam on startup**: Use default `--include-existing=false`
3. **Tool output noise**: Adjust with `--tool-display none`

## Installation & Distribution

```bash
# Install from crates.io
cargo install claude-logger

# Install from local source
cargo install --path .
```

## Future Extension Points

- Additional webhook formats can be added to `WebhookFormat` enum
- New output formatters can extend `LogFormatter`
- Alternative file watching strategies in `LogWatcher`
- Custom message filters in `process_jsonl_file`