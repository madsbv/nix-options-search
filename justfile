alias ld := log-debug
log-debug:
    RUST_LOG="nox=DEBUG" cargo run
