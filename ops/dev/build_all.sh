#!/bin/bash
set -euo pipefail

SCRIPT_DIR=$(dirname $0)
cd "$SCRIPT_DIR/../.."
REPO_DIR="$( pwd )"

cd "$REPO_DIR/proto"
mkdir -p "$REPO_DIR/indexers/go/pb"
protoc \
    --go_out=../indexers/go/pb --go_opt=paths=source_relative \
    --python_out=../indexers/python/territory_python_scanner \
    uim.proto

cd "$REPO_DIR/core"
wasm-pack build --target web  # build WebAssembly
cd pkg
yarn link

cd "$REPO_DIR/client"
maturin build -F py     # buld python lib
maturin develop

cd "$REPO_DIR/front"
yarn install
yarn link territory_core
yarn build

cd "$REPO_DIR"
cargo build -r

cd "$REPO_DIR/indexers/go"
go build -o goscan ./main

cd "$REPO_DIR"
pip install -e territory_local_indexer
pip install -e indexers/python
