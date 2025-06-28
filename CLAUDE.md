# Claude Logger

A real-time monitoring tool for Claude Code's JSONL log files that streams conversation content to standard output.

## Project Overview

### Purpose
- Monitor and display Claude Code conversations in real-time
- Build foundation for future Slack integration and work out loud functionality
- Provide mechanism to share development thought processes externally

### Tech Stack
- **Language**: Rust (edition 2024)
- **Async Runtime**: Tokio
- **File Watching**: notify crate (inotify)
- **CLI**: clap
- **Date/Time**: chrono
- **JSON Processing**: serde_json

## Architecture

### Module Structure
```
src/
├── main.rs       - CLI entry point
├── watcher.rs    - File watching logic
├── parser.rs     - JSONL parsing and message extraction
└── formatter.rs  - Output formatting
```

### Data Flow
1. **File Watching**: Detect JSONL file changes using inotify
2. **Parsing**: Parse new lines as JSON and extract messages
3. **Formatting**: Format into user-friendly output
4. **Output**: Display messages to stdout in Japanese

## Configuration & Customization

### Log File Location
- Default: `~/.claude/projects/[project-name]/[UUID].jsonl`
- Uses `HOME` environment variable to construct paths

### Output Formatting
Customizable in `formatter.rs`:
- Timestamp display toggle
- Session ID display toggle
- Compact mode (truncate long messages)

### Performance Settings
- File watching interval: 100ms
- Async channel buffer size: 100

## Future Roadmap

### Phase 1: Core Features (Complete)
- [x] JSONL file monitoring
- [x] Message parsing & formatting
- [x] CLI interface

### Phase 2: Extended Features
- [ ] Slack webhook integration
- [ ] Message filtering
- [ ] Configuration file support
- [ ] Log level settings

### Phase 3: Work Out Loud
- [ ] Real-time thought process sharing
- [ ] Team dashboard
- [ ] Analytics & statistics

## Development Notes

### Known Issues
- Memory usage with large files
- Resource consumption during multi-project monitoring
- Handling JSONL format changes

### Testing Strategy
- Unit tests: Basic tests implemented in `formatter.rs`
- Integration tests: Need validation with actual JSONL files
- Performance tests: Load testing with high message volume

### Debugging
```bash
# Run with detailed logging
RUST_LOG=debug cargo run -- watch --latest

# Test with specific JSONL file
./target/debug/claude-logger watch -p ~/.claude/projects/-home-suzuki-repos
```

## Design Principles

1. **Simplicity**: Minimal dependencies for reliable operation
2. **Extensibility**: Modular design for future feature additions
3. **Japanese Support**: All output and documentation in Japanese
4. **Real-time**: Immediate detection and processing of file changes
5. **Safety**: Comprehensive error handling to prevent crashes