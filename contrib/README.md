# contrib

Fork-only extras that run on top of the `hcom` CLI and do not touch its code.

## hcom-gui

Tk window listing every catalog agent (`hcom agent list --all --json`) and every live
instance (`hcom list --json`), including instances started with `--as`. Buttons run
existing commands (`hcom agent`, `attach`, `kill`) and log their output; others open the
selected agent's directory in configured applications. The Start dialog sets `--as`,
`--dir`, `--cli` (CLIs installed on PATH), a resume checkbox (`--resume`, or
`--continue` when another CLI is chosen) and a multi-line prompt (Shift+Enter for a new
line, Enter or keypad Enter starts): `--hcom-prompt`, the agent's first message, which
replaces the catalog `prompt` for this launch and leaves `system_prompt` alone; left
empty, the catalog `prompt` (if any) applies.
Send (running instances) asks for a multi-line message the same way and runs
`hcom send -b @<instance> -- <text>` (sender `bigboss`, the human); an empty text sends
nothing.
`Kill group ▾` lists each catalog group with running members (`@persistent (3)`), then
each CLI with running instances (`tool:claude (2)`), and, after confirming the names,
kills exactly those instances. Since is the age of the
current status, Age the time since the instance started; both sort numerically. Context is the input size of the latest model call
and its share of the window, read from the tail of Claude and Codex transcripts (Claude
transcripts lack the window, so 1M is assumed). Quota is the used share of Codex's
primary (5h) rate limit from the same transcript. The Branch and Git columns show each directory's current branch and its
state: `*` uncommitted changes, `↑2 ↓1` commits ahead of/behind the upstream (one
`git status` per directory on every refresh). Where is the terminal an
instance was launched in, with the Herdr workspace/tab (`herdr: cc-hcom/hcom_dev`, `kitty`). A catalog agent running only under other names (`--as`) keeps
its row, not greyed, with Status `running as <names>`. After Start with `--as`, the new
instance is selected once it appears, unless another row was selected meanwhile. Clicking a column heading sorts ascending (▲), again descending
(▼), a third time back to the default order (agent, then instance). Running only hides
catalog rows without an instance; Blocked only lists only instances in `blocked` status.
The Filter field narrows the table as you type to rows whose agent, instance, directory
or catalog groups contain the text (case-insensitive); it combines with both checkboxes.
Sort, window layout and both filters are kept in `~/.hcom/gui-state.json`.

```sh
ln -s "$PWD/contrib/hcom-gui" ~/.local/bin/hcom-gui
hcom-gui
```

Requires Python 3 with tkinter. Tests: `contrib/test-hcom-gui` (needs `xvfb-run`; hcom is
replaced by fixtures, commands are recorded, not run). Optional `~/.hcom/gui.json`:

```json
{
  "refresh_seconds": 5,
  "open_with": [
    {"label": "Zed",  "cmd": ["zed", "{dir}"], "key": "z"},
    {"label": "gitg", "cmd": ["gitg"], "cwd": "{dir}", "key": "g"},
    {"label": "gitk", "cmd": ["gitk", "--all"], "cwd": "{dir}", "key": "h"},
    {"label": "Backlog", "cmd": ["x-terminal-emulator", "backlog", "board"], "cwd": "{dir}",
     "if_exists": "{dir}/backlog", "key": "b"}
  ],
  "commands": [
    {"label": "Kill persistent", "cmd": ["hcom", "kill", "@persistent"], "confirm": true},
    {"label": "Kill WDT", "cmd": ["hcom", "kill", "@wdt_all"], "confirm": true, "cwd": "~/projects/wdt"}
  ],
  "columns": ["agent", "instance", "cli", "status", "since", "context", "quota", "dir", "git"]
}
```

`columns` selects and orders the table columns; without it all are shown. Keys: `agent`,
`instance`, `cli`, `status`, `since`, `age`, `context`, `quota`, `detail`, `unread`, `dir`,
`branch`, `git`, `where`, `config`. Unknown keys are reported in the log and skipped.
The `Columns…` button opens a dialog to choose the visible columns and their order
(Space or double-click toggles a column, Up/Down or Ctrl+Up/Down moves it, at least one
stays visible). That choice is a per-user preference saved right away in
`~/.hcom/gui-state.json`; it takes precedence over `columns` here, and Reset drops it.

`if_exists` enables a button only while that path exists (here a Backlog.md project).
`commands` are toolbar buttons that run as written, without a selected row and with the
output in the log; `cwd` (default `~`) decides which project catalogs `@group` resolves in,
`confirm` asks first. `key` on any `open_with` or `commands` entry is a keyboard shortcut
in the table: a Tk event sequence such as `z` or `<F2>`; it runs the button only while
the button is enabled. The button label shows it, e.g. `Zed (z)`, `Kill (Del)`,
`Kill all (Ctrl+F2)`; built-in keys too: `Refresh (F5)`.
`open_with` programs start independently, also while an hcom command runs; one that exits
non-zero within 5 seconds writes its exit code and stderr to the log.
Without `open_with`, the toolbar offers `xdg-open` and `x-terminal-emulator`. Further hcom
buttons go into `BUTTONS` at the top of the script.
