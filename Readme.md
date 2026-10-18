# Devpane

**Run your stack. Stay in control.**

Devpane is an interactive development process manager and terminal workspace. It makes it simple to run, observe, and interact with the multiple services powering your applications — from frontend dev servers and backend APIs to workers and background databases — both for human developers and the AI coding agents working alongside them.

> **Project status:** Under active development (v0.1.0).

---

## What Devpane Solves

Modern applications often require several processes running at the same time:
- Frontend development servers (`bun run dev`, `vite`, `next dev`)
- Backend APIs (`cargo run`, `go run`, `fastapi`)
- Background workers, queues, and watchers
- Databases and auxiliary services

Juggling these in scattered terminal windows is messy. Processes crash silently, output is lost in scrollback, and terminal tabs proliferate. 

Devpane provides:
1. **Interactive Terminal Panes**: A tmux-inspired TUI with VT100 terminal emulation. Switch between service logs with `Tab`, spawn interactive shells with `Ctrl+T`, and scroll smoothly.
2. **Declarative `devpane.yaml`**: Define all project services once. `devpane up` starts your entire stack; `devpane down` shuts it down.
3. **Daemon-Backed Process Supervision**: Runs a background daemon so processes stay alive, logs are persisted, and state is preserved across CLI invocations.
4. **AI-Agent Ready**: Coding agents (Claude, Gemini, Cursor) can manage development services cleanly through structured IPC without hijacking your interactive terminal.
5. **Cross-Platform**: Built for macOS and Windows (supporting Windows Named Pipes, CMD, and PowerShell).

---

### Option 1: 1-Line Quick Install (Recommended)

**macOS & Linux**:
```bash
curl -fsSL https://raw.githubusercontent.com/Ranjeesh0701/devbox/main/scripts/install.sh | sh
```

**Windows (PowerShell as Administrator or User)**:
```powershell
irm https://raw.githubusercontent.com/Ranjeesh0701/devbox/main/scripts/install.ps1 | iex
```

This automatically downloads the latest release, extracts both `devpane` and `daemon`, and configures them into your `PATH`.

---

### Option 2: Pre-built Binaries & Desktop App (GitHub Releases)

Download the latest release for your platform from [GitHub Releases](https://github.com/Ranjeesh0701/devbox/releases):

- **CLI Bundles** (contains both `devpane` and `daemon`):
  - macOS Apple Silicon: `devpane-darwin-arm64.tar.gz`
  - macOS Intel: `devpane-darwin-x64.tar.gz`
  - Windows x64: `devpane-windows-x64.zip`
- **Desktop Application GUI**:
  - macOS: `.dmg` installer
  - Windows: `.msi` / `.exe` installer

Extract the CLI bundle and move the binaries to any folder in your `PATH` (e.g. `/usr/local/bin` or `C:\Windows\System32`).

---

## Quick Start

### 1. Create a `devpane.yaml` in your project

```yaml
version: 1
services:
  backend:
    enabled: true
    command: cargo run
    working_dir: .

  frontend:
    enabled: true
    command: bun run dev
    working_dir: ./website
```

### 2. Launch the workspace

```bash
devpane up
```

This launches the interactive terminal workspace:
- **`Tab` / `Shift+Tab`**: Switch between running service logs
- **`Ctrl+T`**: Open a new interactive shell tab (PTY session)
- **`Ctrl+W`**: Close current shell tab
- **`PageUp` / `PageDown`**: Scroll log output
- **`Ctrl+Q`**: Exit workspace (or `q` while on read-only service tabs)

### 3. Stop your project

```bash
devpane down
```

---

## CLI Reference

| Command | Description |
|---|---|
| `devpane up` | Start all enabled services from `devpane.yaml` and open the workspace |
| `devpane down` | Stop all services associated with the current project directory |
| `devpane list` | List all active process sessions, PIDs, and statuses |
| `devpane start <name> <cmd> [args...]` | Start an individual background process |
| `devpane stop <session_id>` | Stop a specific process |
| `devpane logs <session_id> [--follow]` | View or stream logs for a process |
| `devpane shutdown` | Terminate all supervised processes and stop the daemon |

---

## Desktop App & Website

- **Desktop Interface (`desktop/`)**: Native Tauri GUI for monitoring running sessions and viewing streaming logs.
- **Website (`website/`)**: Modern landing page built with Bun and Vite.

## Contributing

Devbox is evolving incrementally. Contributions, bug reports, and ideas around process management, developer experience, and AI-assisted workflows are welcome.

Before contributing:

1. Check the current implementation and open issues.
2. Keep changes focused and easy to review.
3. Include clear reproduction steps for bugs.
4. Avoid claiming integrations or functionality that are not implemented.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

**Devbox — a clearer view of the processes behind your code.**
