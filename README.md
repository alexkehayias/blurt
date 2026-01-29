# macOS Notification Daemon

*THIS CODE IS AI GENERATED. DO NOT USE.*

A Rust daemon that reads notifications from the macOS system notification database.

## Overview

This daemon connects to the macOS system notification database located at:
`~/Library/Group\ Containers/group.com.apple.usernoted/db2/db`

It reads notification data from the `record` table and attempts to parse the binary plist data into structured information.

## Features

- Asynchronous database operations using `tokio` and `tokio-rusqlite`
- Binary plist decoding using the `plist` crate
- Proper error handling for database access issues
- Support for both absolute paths and `~` expansion

## Requirements

- Rust 1.70 or later
- macOS system with notification database access
- Full Disk Access permission (required for reading the notification database)

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Run with default database path
./target/release/blurt

# Run with custom database path
./target/release/blurt /path/to/custom/database.db
```

## Database Schema

The daemon queries the `record` table and expects binary plist data in the `data` column.

## Permission Requirements

Due to macOS security restrictions, this daemon requires:
1. Full Disk Access permission in System Preferences > Security & Privacy > Privacy > Full Disk Access
2. The application must be added to this list to access the notification database

## Building

```bash
# Build for development
cargo build

# Build for release
cargo build --release
```

## License

This project is licensed under the MIT License.
