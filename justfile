test:
    cargo nextest r --workspace --no-fail-fast
test-one package name:
    RUST_LOG=info cargo test -p {{package}} {{name}}