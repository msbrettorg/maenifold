#!/bin/zsh
# exec $HOME/.npm-global/bin/claude --allow-dangerously-skip-permissions --append-system-prompt "$(<"$HOME/maenifold/assets/usage/PROMPT.md")" "$@"
exec $HOME/.npm-global/bin/claude --allow-dangerously-skip-permissions --append-system-prompt "$(<"$HOME/maenifold/assets/usage/PROMPT.md")" --plugin-dir $HOME/src/ma-collective/maenifold/integrations/claude-code/plugin "$@"