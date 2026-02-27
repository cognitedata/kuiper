CARGO_PROFILE_RELEASE_PANIC="abort" cargo +nightly rustc --release -Z build-std=core,alloc --target x86_64-unknown-linux-none -- -C relocation-model=static
../target/x86_64-unknown-linux-none/release/kuiper_lang_no_std_test
