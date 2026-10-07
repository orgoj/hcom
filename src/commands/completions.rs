//! `hcom completions` — generate shell completion scripts.
//!
//! Supports bash, zsh, and fish completions for all hcom commands,
//! subcommands, running instances, catalog agents, and groups.

use anyhow::{Result, bail};

pub fn help_text() -> &'static str {
    "Usage:
  hcom completions [bash|zsh|fish]

Generate shell completion scripts for the hcom CLI.

Shells:
  bash   Bash completion script (uses complete -F _hcom hcom)
  zsh    Zsh completion function (#compdef hcom)
  fish   Fish completion script

Examples:
  # Bash (eval in ~/.bashrc or add to ~/.bashrc.d/)
  eval \"$(hcom completions bash)\"

  # Zsh (save to $fpath directory, e.g. ~/.zsh/completions/_hcom)
  hcom completions zsh > ~/.zsh/completions/_hcom

  # Fish
  hcom completions fish > ~/.config/fish/completions/hcom.fish"
}

pub fn bash_completion_script() -> &'static str {
    r#"_hcom() {
    local cur prev words cword
    _init_completion 2>/dev/null || {
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
        words=("${COMP_WORDS[@]}")
        cword=$COMP_CWORD
    }

    # Extract global flags and find subcommand
    local cmd=""
    local cmd_idx=0
    local i=1
    while [[ $i -lt $cword ]]; do
        local w="${words[i]}"
        case "$w" in
            --name)
                ((i++)) # skip name value
                ;;
            --go|--new-terminal)
                ;;
            -*)
                ;;
            *)
                cmd="$w"
                cmd_idx=$i
                break
                ;;
        esac
        ((i++))
    done

    # Top-level completion (before command is determined)
    if [[ -z "$cmd" ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "--name --go --help -h --version -v --new-terminal" -- "$cur") )
        else
            local cmds="ack agent archive bundle completions completion config events hooks kill list listen relay reset run send start status stop term transcript update r f claude gemini codex opencode kilo pi omp antigravity agy cursor kimi copilot hermes"
            COMPREPLY=( $(compgen -W "$cmds" -- "$cur") )
        fi
        return 0
    fi

    # Numeric count before tool launch, e.g. `hcom 3 claude`
    if [[ "$cmd" =~ ^[0-9]+$ ]]; then
        if [[ $cword -eq $((cmd_idx + 1)) ]]; then
            local tools="claude gemini codex opencode kilo pi omp antigravity agy cursor kimi copilot hermes"
            COMPREPLY=( $(compgen -W "$tools" -- "$cur") )
            return 0
        fi
        cmd="${words[cmd_idx+1]}"
        cmd_idx=$((cmd_idx + 1))
    fi

    local sub_pos=$((cword - cmd_idx))

    case "$cmd" in
        agent)
            case "$prev" in
                --cli|--tool)
                    COMPREPLY=( $(compgen -W "claude gemini codex opencode kilo pi omp antigravity agy cursor kimi copilot hermes" -- "$cur") )
                    return 0
                    ;;
                --terminal)
                    COMPREPLY=( $(compgen -W "herdr tmux kitty wezterm alacritty ghostty iTerm2 gnome-terminal xterm" -- "$cur") )
                    return 0
                    ;;
                completions)
                    COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
                    return 0
                    ;;
            esac

            local agent_sub="${words[cmd_idx+1]}"
            if [[ $sub_pos -eq 1 ]]; then
                if [[ "$cur" == @* ]]; then
                    COMPREPLY=( $(compgen -W "$(hcom agent list --groups 2>/dev/null)" -- "$cur") )
                elif [[ "$cur" == -* ]]; then
                    COMPREPLY=( $(compgen -W "--help -h" -- "$cur") )
                else
                    local subcmds="list show attach edit completions"
                    local names="$(hcom agent list --names 2>/dev/null)"
                    local groups="$(hcom agent list --groups 2>/dev/null)"
                    COMPREPLY=( $(compgen -W "$subcmds $names $groups" -- "$cur") )
                fi
                return 0
            fi

            case "$agent_sub" in
                list)
                    if [[ "$cur" == @* ]]; then
                        COMPREPLY=( $(compgen -W "$(hcom agent list --groups 2>/dev/null)" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -W "--all --local --json --names --groups --for-agents --for-humans $(hcom agent list --groups 2>/dev/null)" -- "$cur") )
                    fi
                    ;;
                show)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "--catalog --no-project --as --terminal" -- "$cur") )
                    else
                        COMPREPLY=( $(compgen -W "$(hcom agent list --names 2>/dev/null)" -- "$cur") )
                    fi
                    ;;
                attach)
                    COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null) $(hcom agent list --names 2>/dev/null)" -- "$cur") )
                    ;;
                edit)
                    COMPREPLY=( $(compgen -W "--project" -- "$cur") )
                    ;;
                completions)
                    COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
                    ;;
                *)
                    if [[ "$cur" == -* ]]; then
                        COMPREPLY=( $(compgen -W "--as --clean --resume --continue --catalog --no-project --dry-run --terminal --model --reasoning --prompt --system-prompt --tag --dir --cwd" -- "$cur") )
                    fi
                    ;;
            esac
            ;;

        kill|stop)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "-a --all -f --force -t --timeout --reason" -- "$cur") )
            elif [[ "$cur" == @* ]]; then
                COMPREPLY=( $(compgen -W "$(hcom agent list --groups 2>/dev/null)" -- "$cur") )
            else
                local live="$(hcom list --names 2>/dev/null)"
                local grps="$(hcom agent list --groups 2>/dev/null)"
                COMPREPLY=( $(compgen -W "$live $grps all" -- "$cur") )
            fi
            ;;

        send)
            case "$prev" in
                --intent)
                    COMPREPLY=( $(compgen -W "request inform ack" -- "$cur") )
                    return 0
                    ;;
                --level)
                    COMPREPLY=( $(compgen -W "info warn error" -- "$cur") )
                    return 0
                    ;;
                --from)
                    local names="$(hcom list --names 2>/dev/null; hcom agent list --names 2>/dev/null)"
                    COMPREPLY=( $(compgen -W "$names" -- "$cur") )
                    return 0
                    ;;
                --file)
                    _filedir 2>/dev/null || COMPREPLY=( $(compgen -f -- "$cur") )
                    return 0
                    ;;
            esac

            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--intent --reply-to --thread --file --base64 --from --timeout --wait --level --device --ack --no-autostart --quiet --json --spawn-as --as --" -- "$cur") )
            elif [[ "$cur" == @* ]]; then
                local live="$(hcom list --names 2>/dev/null)"
                local catalog="$(hcom agent list --names 2>/dev/null)"
                local grps="$(hcom agent list --groups 2>/dev/null)"
                local targets="@all"
                for n in $live $catalog; do targets="$targets @$n"; done
                targets="$targets $grps"
                COMPREPLY=( $(compgen -W "$targets" -- "$cur") )
            else
                local live="$(hcom list --names 2>/dev/null)"
                local catalog="$(hcom agent list --names 2>/dev/null)"
                local grps="$(hcom agent list --groups 2>/dev/null)"
                local targets=""
                for n in $live $catalog; do targets="$targets @$n"; done
                targets="$targets $grps"
                COMPREPLY=( $(compgen -W "$targets --intent --file --from --quiet --json" -- "$cur") )
            fi
            ;;

        list)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "-v --verbose --json --names --format -a --all -c --current -z --zombies --stopped --sh" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "self $(hcom list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        status)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--json -a --all --clean" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        term)
            case "$prev" in
                inject)
                    COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null)" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "-f --follow --raw --strip-ansi --json --headless --enter --no-enter --key" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "inject feed $(hcom list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        transcript)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--last --full --json --all" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "search $(hcom list --all --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        r|resume)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--model --effort --terminal --as --dir --tag --system-prompt --dry-run --go --headless" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(hcom list --all --names 2>/dev/null) $(hcom agent list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        f|fork)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--name --as --model --effort --terminal --dir --tag --dry-run --go --headless" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        start)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--as --clean --dry-run --no-pty --terminal --tag --dir" -- "$cur") )
            fi
            ;;

        ack)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--all --last" -- "$cur") )
            else
                COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null)" -- "$cur") )
            fi
            ;;

        listen)
            case "$prev" in
                --type)
                    COMPREPLY=( $(compgen -W "message status life" -- "$cur") )
                    return 0
                    ;;
                --status)
                    COMPREPLY=( $(compgen -W "listening active blocked" -- "$cur") )
                    return 0
                    ;;
                --intent)
                    COMPREPLY=( $(compgen -W "request inform ack" -- "$cur") )
                    return 0
                    ;;
                --agent|--from|--participant|--mention)
                    COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null; hcom agent list --names 2>/dev/null)" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--agent --from --participant --mention --type --status --cmd --file --intent --thread --after --before --timeout" -- "$cur") )
            fi
            ;;

        events)
            case "$prev" in
                --type)
                    COMPREPLY=( $(compgen -W "message status life" -- "$cur") )
                    return 0
                    ;;
                --status)
                    COMPREPLY=( $(compgen -W "listening active blocked" -- "$cur") )
                    return 0
                    ;;
                --action)
                    COMPREPLY=( $(compgen -W "created started ready stopped batch_launched launch_failed launch_blocked" -- "$cur") )
                    return 0
                    ;;
                --intent)
                    COMPREPLY=( $(compgen -W "request inform ack" -- "$cur") )
                    return 0
                    ;;
                --agent|--from|--participant|--mention|--idle|--blocked)
                    COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null; hcom agent list --names 2>/dev/null)" -- "$cur") )
                    return 0
                    ;;
                sub)
                    COMPREPLY=( $(compgen -W "list --once --for --device" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--last --all --wait --sql --remote-fetch --device --idle --blocked --agent --type --status --context --action --cmd --file --collision --from --participant --mention --intent --thread --after --before" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "launch sub" -- "$cur") )
            fi
            ;;

        config)
            case "$prev" in
                -i|--instance)
                    COMPREPLY=( $(compgen -W "$(hcom list --names 2>/dev/null; hcom agent list --names 2>/dev/null)" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--json --edit --unset --reset --info --setup -i --instance" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                local keys="tag hints notes timeout subagent_timeout continue_last terminal terminal_preset claude_args gemini_args codex_args opencode_args kilo_args pi_args omp_args antigravity_args cursor_args kimi_args copilot_args hermes_args dev_root auto_approve HCOM_TAG HCOM_HINTS HCOM_NOTES HCOM_TIMEOUT HCOM_SUBAGENT_TIMEOUT HCOM_CONTINUE_LAST HCOM_CLAUDE_ARGS HCOM_GEMINI_ARGS HCOM_CODEX_ARGS HCOM_OPENCODE_ARGS HCOM_KILO_ARGS HCOM_PI_ARGS HCOM_OMP_ARGS HCOM_ANTIGRAVITY_ARGS HCOM_CURSOR_ARGS HCOM_KIMI_ARGS HCOM_COPILOT_ARGS HCOM_HERMES_ARGS"
                COMPREPLY=( $(compgen -W "get set list reset $keys" -- "$cur") )
            fi
            ;;

        hooks)
            case "$prev" in
                --tool)
                    COMPREPLY=( $(compgen -W "claude gemini codex opencode kilo pi omp antigravity cursor kimi copilot hermes" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--tool --all" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "status install uninstall list" -- "$cur") )
            fi
            ;;

        bundle)
            if [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "prepare apply list" -- "$cur") )
            fi
            ;;

        archive)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--all --before --days" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "list clear" -- "$cur") )
            fi
            ;;

        reset)
            COMPREPLY=( $(compgen -W "-f --force --hard --dry-run" -- "$cur") )
            ;;

        relay)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--broker --device" -- "$cur") )
            elif [[ $sub_pos -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "status start stop devices pair" -- "$cur") )
            fi
            ;;

        run)
            if [[ $sub_pos -eq 1 ]]; then
                local user_scripts=""
                local sdir="${HCOM_DIR:-$HOME/.hcom}/scripts"
                if [[ -d "$sdir" ]]; then
                    user_scripts="$(command ls "$sdir" 2>/dev/null | sed -E 's/\.(sh|py)$//')"
                fi
                COMPREPLY=( $(compgen -W "confess debate fatcow $user_scripts" -- "$cur") )
            fi
            ;;

        update)
            COMPREPLY=( $(compgen -W "--check --dry-run -f --force" -- "$cur") )
            ;;

        completions|completion)
            COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
            ;;

        claude|gemini|codex|opencode|kilo|pi|omp|antigravity|agy|cursor|kimi|copilot|hermes)
            case "$prev" in
                --terminal)
                    COMPREPLY=( $(compgen -W "herdr tmux kitty wezterm alacritty ghostty iTerm2 gnome-terminal xterm" -- "$cur") )
                    return 0
                    ;;
            esac
            if [[ "$cur" == -* ]]; then
                COMPREPLY=( $(compgen -W "--tag --terminal --dir --model --system-prompt --effort --as --name --clean --dry-run --go --headless" -- "$cur") )
            fi
            ;;
    esac
}
complete -F _hcom hcom
"#
}

pub fn zsh_completion_script() -> &'static str {
    r#"#compdef hcom

_hcom() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    local -a commands
    commands=(
        'ack:Acknowledge messages'
        'agent:Launch named agents from a JSON catalog'
        'archive:Query past hcom sessions'
        'bundle:Structured context packages for handoffs'
        'completions:Generate shell completions'
        'completion:Generate shell completions'
        'config:Get/set global and per-agent settings'
        'events:Query event stream, manage subscriptions'
        'hooks:Add or remove hooks'
        'kill:Kill + close terminal pane'
        'list:Show agents, status, unread counts'
        'listen:Block until message or event arrives'
        'relay:Cross-device sync + relay daemon'
        'reset:Archive and clear database'
        'run:Execute workflow scripts'
        'send:Send message to your buddies'
        'start:Connect to hcom'
        'status:Installation and diagnostics'
        'stop:Disconnect from hcom'
        'term:View/inject into agent PTY screens'
        'transcript:Read another agent conversation'
        'update:Check and apply updates'
        'r:Resume stopped agent'
        'f:Fork agent session'
        'claude:Launch Claude Code agent'
        'gemini:Launch Gemini CLI agent'
        'codex:Launch Codex agent'
        'opencode:Launch OpenCode agent'
        'kilo:Launch Kilo Code agent'
        'pi:Launch Pi agent'
        'omp:Launch Oh My Pi agent'
        'antigravity:Launch Antigravity agent'
        'agy:Launch Antigravity agent'
        'cursor:Launch Cursor agent'
        'kimi:Launch Kimi agent'
        'copilot:Launch Copilot agent'
        'hermes:Launch Hermes agent'
    )

    _arguments -C \
        '--name[Instance name for identity]:name:' \
        '--go[Skip confirmation prompts]' \
        '(-h --help)'{-h,--help}'[Show help]' \
        '(-v --version)'{-v,--version}'[Show version]' \
        '--new-terminal[Open in new terminal]' \
        '1: :->command' \
        '*:: :->args' && return 0

    case $state in
        command)
            _describe -t commands 'hcom command' commands
            ;;
        args)
            local cmd="${line[1]}"
            case "$cmd" in
                agent)
                    _hcom_agent_sub
                    ;;
                kill|stop)
                    local -a live groups
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    groups=(${(f)"$(hcom agent list --groups 2>/dev/null)"})
                    _arguments \
                        '(-a --all)'{-a,--all}'[All running instances]' \
                        '(-f --force)'{-f,--force}'[Force termination]' \
                        '(-t --timeout)'{-t,--timeout}'[Timeout in seconds]:seconds:' \
                        '--reason[Termination reason]:reason:' \
                        '*:target:(all $live $groups)'
                    ;;
                send)
                    local -a live catalog groups targets
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    catalog=(${(f)"$(hcom agent list --names 2>/dev/null)"})
                    groups=(${(f)"$(hcom agent list --groups 2>/dev/null)"})
                    targets=(@all)
                    for n in $live $catalog; do targets+=("@$n"); done
                    targets+=($groups)
                    _arguments \
                        '--intent[Message intent]:intent:(request inform ack)' \
                        '--level[Log level]:level:(info warn error)' \
                        '--reply-to[Reply to event ID]:id:' \
                        '--thread[Thread name]:thread:' \
                        '--file[Message from file]:file:_files' \
                        '--from[Sender identity]:from:($live $catalog)' \
                        '--timeout[Timeout in seconds]:seconds:' \
                        '--wait[Wait for response]' \
                        '--quiet[Suppress feedback]' \
                        '--json[JSON output]' \
                        '*:recipient:($targets)'
                    ;;
                list)
                    local -a live
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    _arguments \
                        '(-v --verbose)'{-v,--verbose}'[Verbose output]' \
                        '--json[JSON output]' \
                        '--names[Names only]' \
                        '(-a --all)'{-a,--all}'[All sessions]' \
                        '(-c --current)'{-c,--current}'[Current session]' \
                        '*:agent:(self $live)'
                    ;;
                r|resume)
                    local -a all catalog
                    all=(${(f)"$(hcom list --all --names 2>/dev/null)"})
                    catalog=(${(f)"$(hcom agent list --names 2>/dev/null)"})
                    _arguments \
                        '--model[Model to use]:model:' \
                        '--effort[Reasoning effort]:effort:' \
                        '--terminal[Terminal preset]:preset:' \
                        '--as[Instance name]:name:' \
                        '*:target:($all $catalog)'
                    ;;
                f|fork)
                    local -a live
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    _arguments \
                        '--name[New instance name]:name:' \
                        '--as[New instance name]:name:' \
                        '--model[Model to use]:model:' \
                        '--effort[Reasoning effort]:effort:' \
                        '*:target:($live)'
                    ;;
                term)
                    local -a live
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    _arguments \
                        '1:action:(inject feed $live)' \
                        '*:args:'
                    ;;
                completions|completion)
                    _arguments '1:shell:(bash zsh fish)'
                    ;;
                config)
                    _arguments \
                        '--json[JSON output]' \
                        '--edit[Open in editor]' \
                        '--reset[Reset config]' \
                        '1:key:(get set list reset tag hints notes timeout terminal dev_root auto_approve)' \
                        '*:value:'
                    ;;
                *)
                    _default
                    ;;
            esac
            ;;
    esac
}

_hcom_agent_sub() {
    local -a subcmds names groups
    subcmds=('list:List agents' 'show:Show agent config' 'attach:Focus agent window' 'edit:Edit catalog' 'completions:Generate completions')
    names=(${(f)"$(hcom agent list --names 2>/dev/null)"})
    groups=(${(f)"$(hcom agent list --groups 2>/dev/null)"})

    _arguments -C \
        '1:subcommand:->subcmd' \
        '*:args:->subargs' && return 0

    case $state in
        subcmd)
            _describe -t subcmds 'agent command' subcmds
            compadd -a names
            compadd -a groups
            ;;
        subargs)
            case "${line[1]}" in
                show)
                    compadd -a names
                    ;;
                attach)
                    local -a live
                    live=(${(f)"$(hcom list --names 2>/dev/null)"})
                    compadd -a live
                    compadd -a names
                    ;;
                list)
                    _arguments \
                        '--all[All agents]' \
                        '--local[Local agents only]' \
                        '--json[JSON output]' \
                        '--names[Names only]' \
                        '--groups[Groups only]'
                    ;;
                completions)
                    _arguments '1:shell:(bash zsh fish)'
                    ;;
            esac
            ;;
    esac
}

compdef _hcom hcom
"#
}

pub fn fish_completion_script() -> &'static str {
    r#"# Fish completion for hcom

set -l commands ack agent archive bundle completions completion config events hooks kill list listen relay reset run send start status stop term transcript update r f claude gemini codex opencode kilo pi omp antigravity agy cursor kimi copilot hermes

# Top-level commands
complete -c hcom -f -n '__fish_use_subcommand' -a "$commands"

# Global flags
complete -c hcom -n '__fish_use_subcommand' -l name -d "Instance name for identity" -r
complete -c hcom -n '__fish_use_subcommand' -l go -d "Skip confirmation prompts"
complete -c hcom -n '__fish_use_subcommand' -s h -l help -d "Show help"
complete -c hcom -n '__fish_use_subcommand' -s v -l version -d "Show version"
complete -c hcom -n '__fish_use_subcommand' -l new-terminal -d "Open in new terminal"

# Agent command
complete -c hcom -n '__fish_seen_subcommand_from agent' -a 'list show attach edit completions (hcom agent list --names 2>/dev/null) (hcom agent list --groups 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from agent; and __fish_seen_subcommand_from completions' -a 'bash zsh fish'
complete -c hcom -n '__fish_seen_subcommand_from agent; and __fish_seen_subcommand_from show' -a '(hcom agent list --names 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from agent; and __fish_seen_subcommand_from attach' -a '(hcom list --names 2>/dev/null)'

# Kill / stop
complete -c hcom -n '__fish_seen_subcommand_from kill stop' -a 'all (hcom list --names 2>/dev/null) (hcom agent list --groups 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from kill stop' -s a -l all -d "All running instances"
complete -c hcom -n '__fish_seen_subcommand_from kill stop' -s f -l force -d "Force kill"

# Send
complete -c hcom -n '__fish_seen_subcommand_from send' -a '@all (for a in (hcom list --names 2>/dev/null; hcom agent list --names 2>/dev/null); echo "@$a"; end) (hcom agent list --groups 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from send' -l intent -a 'request inform ack' -d "Message intent"
complete -c hcom -n '__fish_seen_subcommand_from send' -l level -a 'info warn error' -d "Log level"
complete -c hcom -n '__fish_seen_subcommand_from send' -l file -r -d "Message from file"

# List
complete -c hcom -n '__fish_seen_subcommand_from list' -a 'self (hcom list --names 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from list' -s v -l verbose -d "Verbose output"
complete -c hcom -n '__fish_seen_subcommand_from list' -l json -d "JSON output"
complete -c hcom -n '__fish_seen_subcommand_from list' -l names -d "Names only"

# Resume / Fork
complete -c hcom -n '__fish_seen_subcommand_from r resume' -a '(hcom list --all --names 2>/dev/null) (hcom agent list --names 2>/dev/null)'
complete -c hcom -n '__fish_seen_subcommand_from f fork' -a '(hcom list --names 2>/dev/null)'

# Completions
complete -c hcom -n '__fish_seen_subcommand_from completions completion' -a 'bash zsh fish'
"#
}

pub fn run(args: &[String]) -> Result<i32> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", help_text());
        return Ok(0);
    }

    let shell = args
        .first()
        .map(String::as_str)
        .or_else(|| {
            std::env::var("SHELL").ok().as_deref().and_then(|s| {
                if s.ends_with("/zsh") || s == "zsh" {
                    Some("zsh")
                } else if s.ends_with("/fish") || s == "fish" {
                    Some("fish")
                } else if s.ends_with("/bash") || s == "bash" {
                    Some("bash")
                } else {
                    None
                }
            })
        })
        .unwrap_or("bash");

    match shell {
        "bash" => {
            print!("{}", bash_completion_script());
            Ok(0)
        }
        "zsh" => {
            print!("{}", zsh_completion_script());
            Ok(0)
        }
        "fish" => {
            print!("{}", fish_completion_script());
            Ok(0)
        }
        other => bail!("unsupported shell '{other}' (bash | zsh | fish)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_completion_contains_function_and_command() {
        let script = bash_completion_script();
        assert!(script.contains("_hcom()"));
        assert!(script.contains("complete -F _hcom hcom"));
        assert!(script.contains("ack agent archive"));
    }

    #[test]
    fn zsh_completion_contains_compdef() {
        let script = zsh_completion_script();
        assert!(script.contains("#compdef hcom"));
        assert!(script.contains("compdef _hcom hcom"));
    }

    #[test]
    fn fish_completion_contains_subcommands() {
        let script = fish_completion_script();
        assert!(script.contains("complete -c hcom"));
        assert!(script.contains("__fish_use_subcommand"));
    }

    #[test]
    fn run_rejects_unknown_shell() {
        let err = run(&["powershell".to_string()]).unwrap_err();
        assert!(err.to_string().contains("unsupported shell 'powershell'"));
    }
}
