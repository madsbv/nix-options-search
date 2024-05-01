alias ld := log-debug
log-debug:
    NOX_DATA="./data" RUST_LOG="nox=DEBUG" cargo run

clean:
    rm -rf data

alias cd := clean-debug
clean-debug: clean log-debug
