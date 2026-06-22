#!/bin/bash

# Terminal 1: Rust server
gnome-terminal -- bash -c '
cd /home/ubu/Desktop/auli/auli-server
cargo clean
cargo build --release
./target/release/auli-server
exec bash
'

# Terminal 2: ngrok tunnel
gnome-terminal -- bash -c '
cd /home/ubu/Desktop/auli/auli-server
ngrok http --domain=api.auli.com.br 3000
exec bash
'

# The vector store is now in-process (pure-Rust, persisted under ./vectors). There is no
# separate database service to launch — it starts with the Rust server in Terminal 1.

# Note: Ensure that gnome-terminal is installed and configured to run these commands.
# ./start_all.sh

