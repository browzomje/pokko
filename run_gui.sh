#!/usr/bin/env sh
set -eu

# Legacy config inherited from some npm/pnpm shells is unsupported by npm 11.
unset npm_config_global_ignore_file NPM_CONFIG_GLOBAL_IGNORE_FILE

# Funziona anche lanciandolo con sh da una directory diversa.
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"
for dependency in node npm cargo; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
        printf 'Manca %s. Installa Node.js e Rust per avviare pokko.\n' "$dependency" >&2
        exit 1
    fi
done
if [ ! -x node_modules/.bin/tauri ]; then
    printf 'Installazione delle dipendenze frontend…\n'
    npm ci
fi
case "${1:-}" in
    --release)
        printf 'Compilazione di pokko…\n'
        npm run tauri build -- --no-bundle
        exec ./src-tauri/target/release/pokko
        ;;
    --help|-h)
        printf 'Uso: sh run_gui.sh [--release]\nSenza opzioni: GUI con aggiornamento automatico.\n--release: compila e avvia l’eseguibile ottimizzato.\n'
        exit 0
        ;;
    '') ;;
    *) printf 'Opzione sconosciuta: %s\n' "$1" >&2; exit 1 ;;
esac
# Usa una porta libera e comunica lo stesso indirizzo a Vite e Tauri.
PORT=$(node --input-type=module -e 'import net from "node:net"; const server = net.createServer(); server.listen(0, "127.0.0.1", () => { console.log(server.address().port); server.close(); });')
export PORT
# Compatibilità WebKitGTK su desktop Linux con driver grafici problematici.
if [ "$(uname -s)" = Linux ]; then
    export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
fi
printf 'Avvio di pokko su porta %s. Ctrl+C per chiudere.\n' "$PORT"
exec npm run tauri dev -- --config "{\"build\":{\"devUrl\":\"http://127.0.0.1:$PORT\"}}"
