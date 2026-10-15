# contrib

Fork-only extras that run on top of the `hcom` CLI and do not touch its code.

## hcom-gui

Tk window listing every catalog agent (`hcom agent list --all --json`) and every live
instance (`hcom list --json`), including instances started with `--as`. Buttons run
existing commands (`hcom agent`, `attach`, `kill`) and log their output; others open the
selected agent's directory in configured applications. The Start dialog sets `--as`,
`--dir`, `--cli` (CLIs installed on PATH) and a resume checkbox: `--resume`, or
`--continue` when another CLI is chosen. `Kill group ▾` lists each catalog group with
running members (`@persistent (3)`) and, after confirming the names, kills exactly those
instances. Since is the age of the current status, Age the time since the instance
started; both sort numerically. Context is the input size of the latest model call
and its share of the window, read from the tail of Claude and Codex transcripts (Claude
transcripts lack the window, so 1M is assumed). Quota is the used share of Codex's
primary (5h) rate limit from the same transcript. The Branch and Git columns show each directory's current branch and its
state: `*` uncommitted changes, `↑2 ↓1` commits ahead of/behind the upstream (one
`git status` per directory on every refresh). Where is the Herdr workspace/tab an
instance was launched in.

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
    {"label": "Zed",  "cmd": ["zed", "{dir}"]},
    {"label": "gitg", "cmd": ["gitg"], "cwd": "{dir}"},
    {"label": "gitk", "cmd": ["gitk", "--all"], "cwd": "{dir}"},
    {"label": "Backlog", "cmd": ["x-terminal-emulator", "backlog", "board"], "cwd": "{dir}",
     "if_exists": "{dir}/backlog"}
  ],
  "commands": [
    {"label": "Kill persistent", "cmd": ["hcom", "kill", "@persistent"], "confirm": true},
    {"label": "Kill WDT", "cmd": ["hcom", "kill", "@wdt_all"], "confirm": true, "cwd": "~/projects/wdt"}
  ]
}
```

`if_exists` enables a button only while that path exists (here a Backlog.md project).
`commands` are toolbar buttons that run as written, without a selected row and with the
output in the log; `cwd` (default `~`) decides which project catalogs `@group` resolves in,
`confirm` asks first, `key` binds a Tk event.
Without `open_with`, the toolbar offers `xdg-open` and `x-terminal-emulator`. Further hcom
buttons go into `BUTTONS` at the top of the script.
