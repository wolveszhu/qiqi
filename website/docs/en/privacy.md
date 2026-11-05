# Privacy & About

Qiqi's design philosophy is simple: **your data belongs to you, and only to you.**

## Our privacy commitments

- **Local first**: all memories, conversations, fact summaries, and settings live in a SQLite database on your machine.
- **Full-database encryption**: the database is encrypted with SQLCipher (full AES), with the key held by your system credentials.
- **Keys never shipped**: the API key required for AI chat is stored only via local encryption — never written in plaintext to frontend code, local storage, or any config file.
- **No networking by default**: except for AI chat you initiate and optional web search, Qiqi sends nothing outward.
- **No active screenshots**: the MVP never adopts active screenshot capabilities that could touch screen privacy.

## About the project

Qiqi is an open-source desktop digital pet built with **Tauri 2 + Vue 3 + TypeScript + PIXI.js**. It uses a "Rust system shell + WebView presentation layer" two-process architecture, refined by the community through "solo full-stack + AI pair programming".

- Repository: <https://github.com/wolveszhu/qiqi>
- License: see the repository LICENSE

If you share the "local-first, privacy-first" belief, we welcome you to build it together.
