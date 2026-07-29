#!/bin/bash
set -e

cargo build --release

mkdir -p ~/.local/bin
cp target/release/signal-bot ~/.local/bin/signal-bot

echo "Successfully installed signal-bot to ~/.local/bin/signal-bot"
echo "Please ensure that ~/.local/bin is in your PATH."
