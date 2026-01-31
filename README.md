# About `blurt`

> WARNING: MOST OF THIS CODE WAS AI GENERATED. IT'S MOSTLY FINE THOUGH.

Stream your macOS notifications.

*Why?*

Instead of integrating with every app and messaging service to get timely data into your AI application, integrate with the one service it's already aggregated—desktop notifications!

But what if a notification isn't delivered e.g. app in focus? Notification deliver reflects what the user sees and what they are actually doing. Do they really need your AI when the user is actively staring at iMessages? Bonus, you're integrated with the Focus features of macOS by default!

## Usage

```bash
blurt
```

Filter by notification type:

```bash
blurt | grep "app.slack.com"
```

Speak your notifications:

```bash
blurt | jq -r --unbuffered '.body' | while read line ; do echo $line | say ; done
```

Forward to another service via webhook (requires compiling with `--feature webhook`):

```bash
blurt https://example.com/webhook
```

## Requirements

- macOS Tahoe (may work on earlier versions but not tested)
- Full Disk Access permission (required for reading the notification database)

## Installation

```bash
cargo install --path .
```

With webhook forwarding:

```bash
cargo install --features webhook --path .
```

## Permission Requirements

Due to macOS security restrictions, this daemon requires:
1. Full Disk Access permission in System Preferences > Security & Privacy > Privacy > Full Disk Access
2. The application must be added to this list to access the notification database


## License

This project is licensed under the MIT License.
