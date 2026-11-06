# hcom-dashboard

Native Rust GUI for visualizing and managing `hcom` agents, instances, and message flows. Built with `egui`/`eframe`.

`hcom-dashboard` integrates interactive 2D node-link graph visualization with full agent fleet management:
- **Layout**: the graph canvas fills the upper left; the right detail panel holds the tabs of the selected agent (Summary, Report (F9), Screen (F5), Transcript (F6), Events (F7), Log (F8), Config) and the graph-wide Legend tab. The agent table spans the full window width at the bottom. The toolbar view selector switches between `Both (F4)`, `Graph`, and `Table` modes (pressing F4 toggles between Both and Graph modes); `Detail` shows or hides the detail panel; an F5–F9 key opens the detail panel on its tab. F-keys act even while a text field has focus. Drag the table top edge or the detail panel left edge to resize (double-click resets to 30 %); a size is stored as a fraction of the window, so it stays put until you drag again and keeps its proportion on another screen. Panel content never changes the size. Widen the detail panel for the Screen tab (terminal lines scroll horizontally, the inject bar stays at the panel bottom). The canvas keeps at least 150 px of height and 200 px of width while the panels need more; the panels never go below 80 px (table) and 240 px (detail panel), so in a very small window the canvas gives way first. The stored fraction stays for the next larger window.
- **Config tab & stopped agents**: When an agent is stopped, the Screen, Transcript, and Events detail tabs show an explicit stopped notice with a `📋 View Config` shortcut to switch to the Config tab. The Config tab displays the agent's catalog attributes from `agents.json` (CLI tool, description, directory, model, reasoning effort, groups, roaming configuration, skills, instructions, status columns) with parameter preview controls for future editing, while adhering to catalog isolation (catalog provenance paths are not exposed).
- **Agent count**: the top bar shows `Shown N / M` (agents the search and filters let through / all known instances and catalog agents; the status counts cover all), explained in its tooltip. The agent Summary leaves out empty fields; Status Age uses the format of the `since` column.
- **Table**: Real-time listing of active instances and catalog agents with configurable columns and search/filtering. A click anywhere in a row selects it (one band over the whole row) and its graph node; Up/Down select the next or previous row in the shown order like a click and scroll it into view (not while a text field has focus; the graph has no keyboard control); selecting a node in the graph also scrolls its row into view. A double-click on a running agent's row, or Enter on the selected one (not while a text field or dialog has the keyboard), attaches it like a double-click on its node. `Filter table` (top bar, after the filter checkboxes, on by default, saved in the state file) limits the table to the agents the search and filters let through, like the graph; off, the table lists every agent (e.g. to start a stopped one) while the graph stays filtered; cells longer than 300 px end with … and show the full value in a tooltip; a click on a column header sorts by it.
- **Graph styling**: dashed spawn edges (parent → child), solid message edges, a CLI-colored ring per node, context window fill from the bottom (green < 70 %, amber 70–85 %, red > 85 %), a dashed outline for roaming agents, and the agent name below the node on a canvas-colored background. When two names overlap, the name of the agent with fewer messages in the window is hidden; the hovered agent, the selected agent and its neighbours always keep their name. The Legend tab of the detail panel explains the status border colors, CLI ring colors, the roaming outline and the edge styles, listing only those visible in the current (filtered) graph.
- **Layout modes** (top bar, persisted): *Hierarchy* arranges the spawn tree in layers (a node's parent is its `parent_name` for Claude subagents, otherwise the agent that launched its current session, `by` of the last `batch_launched` life event, kept across a `hcom handoff` relaunch; sessions the user launched are roots) (parents centered over children, message springs act horizontally; when the root layer is wider than the canvas, roots without children wrap into rows below the trees, shaped to fit the canvas); *Cluster* is force-directed with message springs scaled by ln(1 + count), so heavy communicators sit together; *Manual* stops the simulation and nodes move only by dragging. A drag alone stores nothing; `Pin Position` in a node's right-click menu pins it (a light dot on its upper right), so it keeps its place in every mode and after a restart, and `Unpin Position` releases it. New nodes appear next to their parent or a message partner, otherwise on a spiral; a collision radius keeps nodes from overlapping. `Reset` (right of `Fit`; disabled in *Manual*) rearranges every unpinned node from scratch in the current mode, agents without any edge in rows next to the others in *Cluster*; after a Reset or a switch between modes the nodes glide to the new arrangement in 4 s (ease in and out) and then rest. Pinned nodes stay; unpin them to rearrange them.
- **Fit to content**: `Fit` (top bar, right) zooms and pans so every shown agent, its rings and its label fit the canvas with a 5 % margin (up to 400 %). Without a saved view, or when the saved view shows no agent, the view fits itself at start and keeps following the layout (up to 100 %) until you zoom, pan or drag a node; it never does so in *Manual* or while a mouse button is held. A triangle on the canvas edge, in its status color, points to every shown agent outside the view.
- **Theme**: the button right of `Fit` (top bar) cycles `💻 Auto` → `☀ Light` → `🌙 Dark`; the choice is saved in the state file. *Auto* follows the theme the windowing system reports; where it reports none (e.g. LXQt on X11), it reads the desktop config once at start: the `window_color` of the LXQt `[Palette]` in `lxqt/lxqt.conf`, else `gtk-3.0/settings.ini` (`gtk-application-prefer-dark-theme`, or a `gtk-theme-name` containing `dark`), both under `$XDG_CONFIG_HOME` or `~/.config`. Without any source it is dark.
- **Message time window**: presets (5m, 15m, 1h, 6h, 24h, All), an interactive two-handle range slider over retained history (left thumb drags start time, right thumb drags end time; boundary snapping activates All or Live), a Live toggle, a text input field accepting relative spans (e.g. `1h`, `15m`, `all`) or absolute intervals (e.g. `12:00 - 13:00`, `2026-11-03 12:00 - 2026-11-03 13:00`, `all - 13:00`, `12:00 - now`; invalid inputs are visibly marked in red leaving the interval untouched), and a "Last N msgs" limit (0 shows `no limit`). The slider tooltip names the window end: `now` while Live is on, otherwise the end time in UTC. Edge badges (`1 msg`, `N msgs`), edge thickness and the conversation detail follow the window; a badge shows from 2 messages on, or on any edge of the selected or hovered agent; edges without messages in it are dimmed but stay clickable. The header shows the range in UTC. At start the graph loads every message of the current hcom database (archived sessions excluded) and keeps all of them, so counts and history are exact for any range; only an explicit "Last N msgs" limits the number of messages, in Live too. The conversation detail lists the newest 50 messages of the window. In a Live window, an edge flashes in the selection color for 0.4 s when a new message arrives on it (not for the history loaded at start).
- **Hover tooltip**: hovering a node shows a card beside it: right, left, below or above, the first position that fits the canvas without covering the node. The card is as wide as its content (at most 360 px) and fades out when the pointer leaves the node. It is hidden while dragging, with a mouse button pressed, and while a menu or dialog is open. Fields come from the `tooltip` config key, using the same keys as `report` (including `status:<name>`), in the listed order; the default is `["status", "detail", "cli", "since", "age", "dir", "unread", "context"]`, and `[]` turns the tooltip off. Empty values are skipped, and a multi-line value shows its first line with `…`; `dir` shows the home directory as `~` and a path longer than 48 characters with `…` in the middle.
- **Usage columns**: `context` (tokens and percent of the context window of the latest model call) for Claude and Codex, and `quota` (Codex primary rate limit, used percent), read from the last 1 MiB of the instance's transcript on each table refresh. Claude transcripts record no window size; 1M is assumed.
- **Idle column**: once a listening instance has been listening for `idle_seconds` (default 10) and the column is shown, `contrib/idle-classifier/idle_check.py` classifies it as `needs_user`, `waiting` or `done` (`-` none, `…` queued), once per listening period; at most one check (up to 32 agents, 180 s timeout) runs at a time. Remote instances report no listening age: they count from first sight. Errors appear in the command log as idle_check's own error line only.
- **Actions**: Start catalog agents, Send messages, Attach/focus terminal panes, Restart, Kill individual agents or groups, and launch external tools (`open_with`).

---

## Configuration & State Persistence

`hcom-dashboard` keeps its configuration and state in its own files:

### Configuration
`~/.hcom/dashboard.json` (same keys as `contrib/hcom-gui`'s `gui.json`). If it is missing or its syntax is corrupt, built-in defaults are used.

### State Persistence
User layout preferences (table and detail panel visibility, active detail tab, table height and detail panel width as window fractions, active filters, sort order, layout mode, the message window span and limit, zoom, pan, and the positions of pinned nodes) are saved to:
```
~/.hcom/dashboard-state.json
```

- A pinned node's position (`pinned_positions`) is stored by instance name when it is pinned and when a drag of the pinned node ends; the 500 most recently stored pins are kept. A pinned node starts there and the simulation does not move it in any mode, *Hierarchy* included.
- Zoom, pan and the split fractions are saved 1 s after the last change (no mouse button held) and on a normal exit; a kill can lose the last second.
- Invalid values are dropped or clamped on load and before saving (non-finite numbers, zoom outside 15–400 %, split fraction outside 0–1 (dropped) or outside 5–95 % (clamped), an unknown detail tab becomes Summary); a bad position entry is skipped without rejecting the file, and an unreadable `dashboard-state.json` falls back to defaults.
- Several open `hcom-dashboard` windows share the file: each save writes a unique temp file and renames it, so the file stays valid and the last writer wins.

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
