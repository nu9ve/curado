#!/bin/zsh
# Optional engine for stems, in its own environment outside the app. Needs uv and Apple Silicon.
# Optional argument: the Python to use, such as scripts/setup-separation.sh 3.11.
set -euo pipefail
repo="${0:A:h:h}"
target="$HOME/Library/Application Support/studio.curado.player/separation"
if [[ ! -x "$target/bin/python" ]]; then
    uv venv --python "${1:-3.10}" "$target"
fi
uv pip install --python "$target/bin/python" -r "$repo/scripts/requirements-separation.txt"
echo "Listo. Abre curado: Configuración → Audio → Stems. Los modelos se descargan al primer uso."
