# hcom-graph

Native Rust GUI for visualizing and managing `hcom` agents, instances, and message flows. Built with `egui`/`eframe`.

`hcom-graph` integrates interactive 2D node-link graph visualization with full agent fleet management:
- **Table view**: Real-time listing of active instances and catalog agents with configurable columns and search/filtering.
- **Inspect panels**: Screen (interactive PTY viewer with shortcuts), Transcript, Events, Log, and structured Report.
- **Actions**: Start catalog agents, Send messages, Attach/focus terminal panes, Restart, Kill individual agents or groups, and launch external tools (`open_with`).

---

## Configuration & State Persistence

`hcom-graph` adheres to strict configuration and state isolation rules:

### Configuration Precedence
1. `~/.hcom/graph.json` (takes precedence if present).
2. `~/.hcom/gui.json` (fallback, fully compatible with `contrib/hcom-gui`).

If neither exists or if syntax is corrupt, built-in defaults are used.

### State Persistence
User layout preferences (view mode, active filters, sort order, and active bottom tab) are saved to:
```
~/.hcom/graph-state.json
```
> **CRITICAL BOUNDARY:** `hcom-graph` **NEVER** modifies or writes to `~/.hcom/gui-state.json`. It only reads `gui-state.json` as a one-way initial fallback if `graph-state.json` does not exist.

---

## Tool Execution & Placeholders

- **External Launchers (`open_with`)**: Run asynchronously in detached mode (`SpawnDetached`) without blocking the GUI or background sync workers. Child processes are automatically reaped.
- **Custom Commands (`commands`)**: Run synchronously bounded via `RunCommand` with a 30-second timeout, with output and exit status captured and displayed in the Log tab.

### Supported Placeholders
Commands support the following token substitutions:
- `{dir}`: Agent working directory.
- `{instance}`: Live instance name.
- `{agent}`: Catalog agent name.
- `{terminal}`: Terminal preset or emulator.
- `{pane_id}`: Terminal pane identifier.
- `{herdr_workspace}`: Herdr workspace.
- `{herdr_tab}`: Herdr tab name.

Any unrecognized `{...}` placeholder is rejected with an error dialog.

### `if_exists` Guard
An `open_with` entry can specify an `"if_exists": "{dir}/..."` path. The button is only enabled/shown when the target file or directory exists on disk.
