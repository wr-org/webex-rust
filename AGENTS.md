# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `webex-rust`, an asynchronous Rust library providing a minimal interface to Webex Teams APIs. It's designed primarily for building bots but supports general API interactions.

## Commands

### Build and Test
- `cargo build` - Build the library
- `cargo test` - Run unit tests  
- `cargo clippy` - Run linter (note: very strict clippy rules enabled)
- `cargo fmt` - Format code
- `cargo doc` - Generate documentation

### Examples
- `cargo run --example hello-world` - Basic message sending example
- `cargo run --example auto-reply` - Bot that automatically replies to messages
- `cargo run --example adaptivecard` - Demonstrates AdaptiveCard usage
- `cargo run --example device-authentication` - Shows device authentication flow

### Development
- `cargo test --lib` - Run library tests only
- `cargo clippy --all-targets --all-features` - Full clippy check
- `cargo build --all-targets` - Build everything including examples

### Git Hooks
- `./hooks/install.sh` - Install pre-commit hooks that automatically run cargo fmt
- Pre-commit hook ensures code is formatted before each commit

## Architecture

### Core Components

- **`Webex` struct** (`src/client/mod.rs`) - Main API client with token-based authentication
- **`WebexEventStream`** (`src/client/websocket.rs`) - WebSocket event stream handler for real-time events
- **`RestClient`** (`src/client/rest.rs`) - Low-level HTTP client wrapper with flexible authentication
- **Client module** (`src/client/`) - Client implementation split into modular components
- **Types module** (`src/types/`) - All API data structures organized by resource type
- **AdaptiveCard module** (`src/adaptive_card/`) - Support for interactive cards with builders
- **Auth module** (`src/auth.rs`) - Device authentication flows (OAuth device grant)
- **Error module** (`src/error.rs`) - Comprehensive error handling

### Key Patterns

- **Generic API methods**: `get<T>()`, `list<T>()`, `delete<T>()` work with any `Gettable` type
- **Device registration**: Automatic device setup and caching for WebSocket connections
- **Message handling**: Supports both direct messages and room messages with threading
- **Event streaming**: WebSocket-based real-time event processing with automatic reconnection

### Authentication Flow

1. Token-based authentication for REST API calls
2. Device registration with Webex for WebSocket connections  
3. Mercury URL discovery for optimal WebSocket endpoint
4. Automatic device cleanup and recreation as needed

## Important Notes

- Uses Rust 1.92 toolchain (see `rust-toolchain.toml`)
- Very strict clippy configuration with pedantic and nursery lints enabled
- All public APIs must have documentation (`#![deny(missing_docs)]`)
- WebSocket connections require device registration and token authentication
- Mercury URL caching reduces API calls for device discovery
- Comprehensive CI workflow with tests, clippy, fmt, build, and doc checks
- Git pre-commit hooks available in `hooks/` directory to auto-format code

## Recent Refactoring (v0.11.0)

- **Module organization**: Refactored large files into logical modules
  - `src/lib.rs` reduced from 1532 lines to 54 lines (thin orchestrator)
  - `src/client/` module split into `mod.rs`, `rest.rs`, and `websocket.rs`
  - `src/types/` module organized by resource type (message, room, person, etc.)
  - `src/adaptive_card/` module split into elements, containers, and styles
- **Backward compatibility**: All public APIs maintained, including Clone trait on Webex struct
- **Test coverage**: 37 unit tests ensuring functionality after refactoring
- **Documentation**: Fixed broken doc links, cargo doc builds with zero warnings