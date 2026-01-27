test:
    cargo nextest r -p gebeh-core -p gebeh --no-fail-fast
test-one package name:
    RUST_LOG=info cargo test -p {{package}} {{name}}
clippy:
    cargo clippy --workspace --all-targets
