# jcode Lab Notes

Setup notes:
- Fork cloned successfully.
- origin points to my fork.
- upstream points to original jcode repo.
- Release build works.
- Debug help command caused a stack overflow on Windows, so for now use target/release/jcode.exe.
- cargo test appeared to hang on some provider/auth/Ollama-related tests, so use cargo check as the basic setup check for now.

Current safe command:
cargo build --release

Current usable binary:
.\target\release\jcode.exe
