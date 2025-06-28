# Claude Logger

Real-time monitoring tool for Claude Code conversations. Watches JSONL log files and streams formatted messages to stdout.

## Features

- 🔍 **Real-time monitoring** of Claude Code JSONL logs
- 📁 **Project management** - monitor latest, specific, or all projects
- 🎯 **Smart filtering** - extracts user and assistant messages only
- 🇯🇵 **Japanese output** - user-friendly formatting with emojis
- ⚡ **Fast & lightweight** - efficient file watching with inotify

## Installation

### Prerequisites
- Rust 1.88.0 or later
- Claude Code installed and configured

### Build from source
```bash
git clone <repository-url>
cd claude-logger
cargo build --release
```

The binary will be available at `./target/release/claude-logger`

## Usage

### List available projects
```bash
claude-logger list
```
Output:
```
利用可能なプロジェクト:
  "-home-suzuki-repos-dotfiles" (18 セッション)
  "-home-suzuki-repos-aocs-all" (1 セッション)
  "-home-suzuki-repos" (1 セッション)
```

### Monitor latest project
```bash
claude-logger watch --latest
```

### Monitor specific project
```bash
claude-logger watch --project-path /home/user/.claude/projects/-home-user-repos
# or short form
claude-logger watch -p /home/user/.claude/projects/-home-user-repos
```

### Monitor all projects
```bash
claude-logger watch --all
```

## Output Format

Messages are displayed with timestamps and role indicators:

```
[14:23:15] 👤 ユーザー: Help me implement a file watcher in Rust
[14:23:18] 🤖 Claude: I'll help you create a file watcher in Rust...
[14:23:20] 🤖 Claude: [ツール使用: Write]
```

## Configuration

### Log File Location
Claude Logger automatically detects log files in:
```
~/.claude/projects/[project-name]/[session-uuid].jsonl
```

### Environment Variables
- `HOME` - Used to locate Claude configuration directory

## Examples

### Basic monitoring workflow
```bash
# 1. Check available projects
claude-logger list

# 2. Start monitoring latest project
claude-logger watch --latest

# 3. In another terminal, start Claude Code session
claude --project /path/to/your/project

# 4. Watch real-time conversation output
```

### Pipe to file or other tools
```bash
# Save to file
claude-logger watch --latest > conversation.log

# Send to Slack (future feature)
claude-logger watch --latest | slack-sender

# Filter specific content
claude-logger watch --latest | grep "ユーザー"
```

## Troubleshooting

### No projects found
- Ensure Claude Code has been run at least once
- Check that `~/.claude/projects/` directory exists
- Verify permissions to read the directory

### File not updating
- Confirm Claude Code is actively running
- Check file permissions
- Try monitoring a different project

### Performance issues
- Large log files may cause memory usage spikes
- Consider monitoring specific projects instead of all projects
- Check system inotify limits if monitoring fails

## Architecture

### Core Components
- **Watcher**: File system monitoring using inotify
- **Parser**: JSONL parsing and message extraction  
- **Formatter**: User-friendly output formatting
- **CLI**: Command-line interface with clap

### Message Flow
```
JSONL File → File Watcher → Parser → Formatter → stdout
```

## Contributing

### Development setup
```bash
# Clone and setup
git clone <repository-url>
cd claude-logger

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- watch --latest

# Build release version
cargo build --release
```

### Code structure
- `src/main.rs` - CLI interface and command handling
- `src/watcher.rs` - File watching and project management
- `src/parser.rs` - JSONL parsing and message extraction
- `src/formatter.rs` - Output formatting and display

## License

[Specify your license here]

## Roadmap

- [ ] Configuration file support
- [ ] Slack webhook integration
- [ ] Message filtering options
- [ ] Web dashboard interface
- [ ] Team collaboration features

## Support

For issues and feature requests, please [open an issue](link-to-issues).

---

Built with ❤️ for the Claude Code community