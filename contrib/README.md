# contrib

Fork-only extras that run on top of the `hcom` CLI and do not touch its code.

## hcom-gui

Tk window listing every catalog agent (`hcom agent list --all --json`) and every live
instance (`hcom list --json`), including instances started with `--as`. Buttons run
existing commands (`hcom agent`, `attach`, `kill`) and log their output; others open the
selected agent's directory in configured applications. The Start dialog sets `--as`,
`--dir`, `--cli` (CLIs installed on PATH) and a resume checkbox: `--resume`, or
`--continue` when another CLI is chosen.

```sh
ln -s "$PWD/contrib/hcom-gui" ~/.local/bin/hcom-gui
hcom-gui
```

Requires Python 3 with tkinter. Optional `~/.hcom/gui.json`:

```json
{
  "refresh_seconds": 5,
  "open_with": [
    {"label": "Zed",  "cmd": ["zed", "{dir}"]},
    {"label": "gitg", "cmd": ["gitg"], "cwd": "{dir}"},
    {"label": "gitk", "cmd": ["gitk", "--all"], "cwd": "{dir}"}
  ]
}
```

Without `open_with`, the toolbar offers `xdg-open` and `x-terminal-emulator`. Further hcom
buttons go into `BUTTONS` at the top of the script.
