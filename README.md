# Claude Logger

Real-time monitoring tool for Claude Code conversations. Watches JSONL log files and streams formatted messages to stdout with optional webhook integration.

## Features

- 🔍 **Real-time monitoring** of Claude Code JSONL logs
- 📁 **Project management** - monitor latest, specific, or all projects
- 🎯 **Smart filtering** - skip historical messages by default
- 🔧 **Tool display modes** - hide, simplify, or detail tool usage
- 🔔 **Webhook integration** - send messages to Slack or custom endpoints
- ⚡ **Fast & lightweight** - efficient file watching with inotify

## Installation

```bash
git clone <repository-url>
cd claude-logger
cargo build --release
```

The binary will be available at `./target/release/claude-logger`

## Usage

### Monitor latest project
```bash
claude-logger watch --latest
```

## Advanced Options

### Tool Display Modes
Control how tool usage is displayed:
```bash
# Hide all tool usage (default: simple)
claude-logger watch --latest --tool-display none

# Show simple indicators like "🔧 Bash"
claude-logger watch --latest --tool-display simple

# Show detailed tool usage
claude-logger watch --latest --tool-display detailed
```

### Include Historical Messages
By default, only new messages are shown. To include existing messages:
```bash
claude-logger watch --latest --include-existing
```

### Webhook Integration
Send messages to external services:
```bash
# Slack webhook
claude-logger watch --latest \
  --webhook-url https://hooks.slack.com/services/YOUR/WEBHOOK/URL \
  --webhook-format slack

# Generic JSON webhook
claude-logger watch --latest \
  --webhook-url https://example.com/webhook \
  --webhook-format generic
```

## Output Format

Messages are displayed with timestamps and role indicators:

```
[14:23:15] 👤 User: Help me implement a file watcher in Rust
[14:23:18] 🤖 Claude: I'll help you create a file watcher in Rust...
[14:23:20] 🤖 Claude: 🔧 Write
[14:23:22] 🤖 Claude: ✅ Result
```

### Tool Display Examples

**None mode**: Tool usage is hidden
```
[14:23:15] 👤 User: Create a test file
[14:23:18] 🤖 Claude: I'll create a test file for you.
```

**Simple mode** (default): Shows tool names
```
[14:23:15] 👤 User: Create a test file
[14:23:18] 🤖 Claude: I'll create a test file for you.
[14:23:20] 🤖 Claude: 🔧 Write
[14:23:22] 🤖 Claude: ✅ Result
```

**Detailed mode**: Shows tool parameters
```
[14:23:15] 👤 User: Create a test file
[14:23:18] 🤖 Claude: I'll create a test file for you.
[14:23:20] 🤖 Claude: 🔧 Write: test.txt
[14:23:22] 🤖 Claude: ✅ File created successfully
```

## Configuration

### Log File Location
Claude Logger automatically detects log files in:
```
~/.claude/projects/[project-name]/[session-uuid].jsonl
```

### Environment Variables
- `HOME` - Used to locate Claude configuration directory
- `RUST_LOG` - Set to `debug` for verbose logging

## Examples

### Basic monitoring workflow
```bash
# 1. Start monitoring latest project
claude-logger watch --latest

# 2. In another terminal, start Claude Code session
claude code

# 3. Watch real-time conversation output
```

### Integration examples
```bash
# Save to file
claude-logger watch --latest > conversation.log

# Filter specific content
claude-logger watch --latest | grep "Error"

# Send to Slack with minimal tool output
claude-logger watch --latest \
  --tool-display simple \
  --webhook-url $SLACK_WEBHOOK_URL \
  --webhook-format slack
```

## Webhook Formats

### Generic Format
Sends JSON payload with full message details:
```json
{
  "timestamp": "2025-06-28T14:23:15Z",
  "role": "User",
  "content": "[14:23:15] 👤 User: Hello",
  "session_id": "session-uuid",
  "uuid": "message-uuid"
}
```

### Slack Format
Sends formatted message ready for Slack display:
```json
{
  "text": "[14:23:15] 👤 User: Hello",
  "blocks": [{
    "type": "section",
    "text": {
      "type": "mrkdwn",
      "text": "[14:23:15] 👤 User: Hello"
    }
  }]
}
```

## Troubleshooting

### No projects found
- Ensure Claude Code has been run at least once
- Check that `~/.claude/projects/` directory exists
- Verify permissions to read the directory

### Duplicate messages on startup
- This is normal if using `--include-existing`
- Default behavior skips existing messages
- Messages are filtered by startup timestamp

### Webhook not receiving messages
- Verify webhook URL is accessible
- Check network connectivity
- Look for error messages in terminal output

### Performance issues
- Large log files may cause initial parsing delays
- Consider using `--tool-display none` for better performance
- Monitor specific projects instead of `--all`

## Development

### Build and test
```bash
# Clone repository
git clone <repository-url>
cd claude-logger

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- watch --latest

# Apply lints
cargo clippy

# Format code
cargo fmt
```

### Architecture Overview
- `src/main.rs` - CLI interface and command handling
- `src/watcher.rs` - File watching and project management
- `src/parser.rs` - JSONL parsing and message extraction
- `src/formatter.rs` - Output formatting and display
- `src/webhook.rs` - Webhook integration

