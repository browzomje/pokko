#!/usr/bin/env sh
set -eu

# Legacy config inherited from some npm/pnpm shells is unsupported by npm 11.
unset npm_config_global_ignore_file NPM_CONFIG_GLOBAL_IGNORE_FILE

# Enable running from any working directory.
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"

for dependency in node npm cargo; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
        printf 'Missing %s. Please install Node.js and Rust to run pokko.\n' "$dependency" >&2
        exit 1
    fi
done

if [ ! -x node_modules/.bin/tauri ]; then
    printf 'Installing frontend dependencies…\n'
    npm ci
fi

case "${1:-}" in
    --release)
        printf 'Building pokko release binary…\n'
        npm run tauri build -- --no-bundle
        exec ./src-tauri/target/release/pokko
        ;;
    --help|-h)
        printf 'Usage: sh run_gui.sh [--release]\nDefault: launch GUI dev server with hot reload.\n--release: build and run optimized release binary.\n'
        exit 0
        ;;
    '') ;;
    *) printf 'Unknown option: %s\n' "$1" >&2; exit 1 ;;
esac

# Find an available port and pass it to both Vite and Tauri.
PORT=$(node --input-type=module -e 'import net from "node:net"; const server = net.createServer(); server.listen(0, "127.0.0.1", () => { console.log(server.address().port); server.close(); });')
export PORT

# WebKitGTK compatibility on Linux desktop systems with problematic graphics drivers.
if [ "$(uname -s)" = Linux ]; then
    export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
fi

printf 'Starting pokko on port %s. Press Ctrl+C to stop.\n' "$PORT"
exec npm run tauri dev -- --config "{\"build\":{\"devUrl\":\"http://127.0.0.1:$PORT\"}}"
