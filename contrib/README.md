# contrib

Fork-only extras that run on top of the `hcom` CLI and do not touch its code.

## idle-classifier

Evaluation scripts and metric-only results for classifying an idle agent's last message
(does the user need to act?), and `idle_check.py`, a CLI that lists which listening agents need
a user decision. See `idle-classifier/README.md`.

## hcom-gui

Tk window listing every catalog agent (`hcom agent list --all --json`) and every live
instance (`hcom list --json`), including instances started with `--as`. Buttons run
existing commands (`hcom agent`, `kill`, Restart, Attach as configured per terminal) and log
their output; others open the selected agent's directory in configured applications. The Start dialog sets `--as`,
`--dir`, `--cli` (CLIs installed on PATH), `--model` and `--reasoning` (editable, prefilled
from the catalog; offered are the catalog's models and the levels minimal to max; a value left
at the catalog's or emptied passes no flag, so the catalog applies), a resume checkbox (`--resume`, or
`--continue` when another CLI is chosen) and a multi-line prompt (Shift+Enter for a new
line, Enter or keypad Enter starts): `--hcom-prompt`, the agent's first message, which
replaces the catalog `prompt` for this launch and leaves `system_prompt` alone; left
empty, the catalog `prompt` (if any) applies.
Send (running instances) asks for a multi-line message the same way and runs
`hcom send -b @<instance> -- <text>` (sender `bigboss`, the human); an empty text sends
nothing.
Inject (running instances) asks for a text and an Enter checkbox (checked by default) and
runs `hcom term inject <instance> <text>`, with `--enter` when checked: the text is typed
straight into the agent's terminal (PTY instances only; hcom's error goes to the Log); an
empty text runs nothing.
Restart (running instances of a catalog agent, including `--as` names), after
confirming, runs `hcom agent <agent> --as <instance> --restart`: it kills the instance,
waits for it to exit and launches the agent again under the same name, with the
catalog's resume/continue setting. hcom commands of a roaming agent's instance run in
its directory, since a roaming agent resolves its project from there.
`Kill group ▾` lists each catalog group with running members (`@persistent (3)`), then
each CLI with running instances (`tool:claude (2)`), and, after confirming the names,
kills exactly those instances. Since is the age of the
current status, Age the time since the instance started; both sort numerically. Context is the input size of the latest model call
and its share of the window, read from the tail of Claude and Codex transcripts (Claude
transcripts lack the window, so 1M is assumed). Quota is the used share of Codex's
primary (5h) rate limit from the same transcript. The Branch and Git columns show each directory's current branch and its
state: `*` uncommitted changes, `↑2 ↓1` commits ahead of/behind the upstream (one
`git status` per directory on every refresh). Where is the terminal an
instance was launched in, with the Herdr workspace/tab (`herdr: cc-hcom/hcom_dev`, `kitty`).
The usage columns come from the life and message events of the live database (`hcom events`;
history restarts after `hcom reset`), per instance name, and a catalog row uses its agent name:
Starts counts `created` events, Runtime sums each session from `created` to the next `stopped`
(a running instance's open session counts up to now), Handoffs counts `handoff` events (relaunch
attempts started by `hcom handoff`), Msgs is sent/received messages, Last start the latest
`created`, Stops the `stopped` events as `total: Nk Ni Ne` (killed, idle autokill, exit = every other end: the CLI exited, terminal closed, cleanup), Failed the
`launch_failed` and `launch_blocked` events. A row without any events shows empty cells. Events
reload at most every `stats_seconds` (default 60) in `gui.json`.
Idle classifies an instance that has been listening for `idle_seconds` (default 10) in
`gui.json`: the GUI runs `idle-classifier/idle_check.py --json <names>` (contract v4 model; needs
the local-models setup described in `idle-classifier/README.md`) and shows `needs_user`,
`waiting` or `done`, `…` while queued or running, `-` when no class came back (failure, stale,
no exchange). It runs once per listening period: a status change, or a listening age that
restarted between two refreshes, clears the cell and starts a new period. One run at a time
(instances due meanwhile go into the next run), at most 180 s (then its process group is
killed); each run loads the model (a few seconds, about 2.3 GB VRAM while it runs), unless every
text is already in idle_check's score cache. Right-click on a classified row corrects its class
(`idle_check.py --label`); the cell shows the correction with `✓` once it is recorded. Failures are
logged as idle_check's category, never message text. New runs start only while the column is
shown. Detection follows the refresh interval, so a run starts up to `refresh_seconds` after
`idle_seconds`. Remote (relay) instances report no listening age: they count from the first
refresh that saw them listening, so after a GUI start they wait a full `idle_seconds`, and a
remote listen–work–listen cycle within one refresh is not seen as a new period. Sorting by Idle
orders needs_user, waiting, done, `-`. A needs_user row is highlighted in blue, less loudly than a blocked one
(the Blocked only filter still shows only blocked instances). A catalog agent running only under other names (`--as`) keeps
its row, not greyed, with Status `running as <names>`. After Start with `--as`, the new
instance is selected once it appears, unless another row was selected meanwhile. When the
selected row disappears (Kill, Kill group, the instance ended, a filter hides it), the row
now at the same position in the current sort and filter is selected, or the last row when it
was the last one; after Start or Restart the selection stays on that agent instead. Clicking a column heading sorts ascending (▲), again descending
(▼), a third time back to the default order (agent, then instance). Empty last, on by default,
puts rows whose cell in the sort column is empty (e.g. stopped agents when sorting by Instance)
after all others, ascending and descending; the default order is not affected. Running only hides
catalog rows without an instance; Blocked only lists only instances in `blocked` status.
A disabled catalog agent's row (`"disabled": true`) is struck through in light grey; Hide
disabled, on by default, hides it. A running instance of a disabled agent stays visible.
Hide non-persistent hides instances that no catalog agent launched (empty Agent column).
The Roaming column marks a roaming catalog agent (`"roaming": true`) and each of its
running instances with `↻`. A roaming agent's catalog row always stays a separate row next
to its instances (status `running as …`), as the archetype for other projects.
The Filter field narrows the table as you type to rows whose agent, instance, directory
or catalog groups contain the text (case-insensitive); it combines with the checkboxes.
The toolbar holds the buttons, wrapped into as many rows as the window width needs (also
after each button named in `toolbar_breaks`), and below them a row with the count of rows the
table shows after all filters, the Filter field and the filter checkboxes.
In the Start dialog the Name field is empty; when it gets focus while empty it is prefilled
with `<agent>__` (clone naming convention), and a name left at exactly that prefix counts as
empty (the default name).
Up/Down move through the rows from anywhere in the main window (e.g. the Filter field)
and give the table keyboard focus.
The bottom panel has tabs Screen, Transcript, Events and Log. For the selected row's running
instance, Screen shows `hcom term <instance>`, Transcript `hcom transcript <instance>` and
Events `hcom events --agent <instance> --last 50` as one `HH:MM:SS type detail` line per event;
a row without a running instance leaves them empty. Only the open tab loads, in the background,
on selection change and on every refresh; it stays at the end unless you scrolled away, and a
failed command shows its error in the tab. Log holds command output and errors; a new ERROR
line while another tab is open marks the Log tab with a red dot, and opening Log clears it.
Sort, Empty last, window layout, the bottom tab and the filters are kept in `~/.hcom/gui-state.json`.

```sh
ln -s "$PWD/contrib/hcom-gui" ~/.local/bin/hcom-gui
hcom-gui
```

Requires Python 3 with tkinter. Tests: `contrib/test-hcom-gui` (needs `xvfb-run`; hcom is
replaced by fixtures, commands are recorded, not run). Optional `~/.hcom/gui.json`:

```json
{
  "refresh_seconds": 5,
  "stats_seconds": 60,
  "idle_seconds": 10,
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
  "columns": ["agent", "instance", "cli", "status", "since", "context", "quota", "dir", "git"],
  "toolbar_breaks": ["Kill group"],
  "attach": {
    "herdr": ["sh", "-c", "hcom agent attach \"$1\" && wmctrl -x -a herdr.herdr", "_", "{instance}"]
  }
}
```

`attach` maps the terminal an instance runs in (the Where column: `herdr`, `tmux`,
`kitty`, ...) to the command of the Attach button (also Enter or double-click on a
running row). Terminals without an entry run `hcom agent attach {instance}`, which
focuses a Herdr pane but not the window of the terminal Herdr runs in, and fails (in the
log) for instances outside Herdr, e.g. a Hermes gateway or a tmux pane. The example
above also raises the Herdr window by its WM_CLASS on X11 (Herdr started with
`wezterm start --class herdr -- herdr`); use whatever raises a window on your desktop,
and add entries such as `tmux` for other terminals. Placeholders: `{instance}`, `{agent}`, `{dir}`, `{terminal}`
and every text field of the instance's `launch_context` (`{pane_id}`,
`{herdr_workspace}`, ...); a placeholder the instance lacks logs an error and runs
nothing, as does an entry that is not a list of strings.

`columns` selects and orders the table columns; without it all are shown. Keys: `agent`,
`instance`, `roaming`, `cli`, `status`, `since`, `idle`, `age`, `context`, `quota`, `detail`, `unread`, `dir`,
`branch`, `git`, `where`, `config`. Unknown keys are reported in the log and skipped.
The `Columns…` button opens a dialog to choose the visible columns and their order
(Space or double-click toggles a column, Up/Down or Ctrl+Up/Down moves it, at least one
stays visible). That choice is a per-user preference saved right away in
`~/.hcom/gui-state.json`; it takes precedence over `columns` here, and Reset drops it. A column
added to hcom-gui after that choice is shown in its default place; hide it in the dialog if unwanted.

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
`toolbar_breaks` lists button labels (built-in such as `Kill group`, `Columns`, `Refresh`,
or `open_with`/`commands` labels; without the key, `…` or `▾` shown on the button) after which the
buttons continue on a new row, e.g. a fixed layout for a portrait monitor; the rows still
wrap when the window is narrower. An unknown label is reported in the log.
