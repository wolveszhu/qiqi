# FAQ

### Is Qiqi free?

Qiqi itself is completely free. The AI chat requires your own OpenAI-compatible API key, or a built-in quota if provided.

### Is my chat history and memory safe?

Very safe. All messages, facts, summaries, and settings are stored in a local SQLite database encrypted with SQLCipher. The key is kept in your system credentials; data is **never uploaded to any cloud**, and API keys are never written to the frontend or stored in plaintext.

### Does Qiqi keep consuming resources?

No. Built on a Rust system shell with a WebView renderer, it has minimal CPU usage when idle and drops to a lower frame rate to save power.

### Why can't I see the pet window?

The pet window is transparent and always on top in a screen corner, and may be covered by other windows. Try moving the foreground window, or adjust its position and visibility in the settings panel.

### Does it support macOS / Linux?

Currently only Windows 10 and later (x64) are supported. Other platforms are on the roadmap.

### How do I switch cat / dog avatars?

Open the settings panel and switch the built-in avatar from the "Avatar" option; custom avatar support will arrive in a later release.

### How do I quit the app?

Qiqi stays resident in the background; closing the settings window only hides it. The real exit entry is in the system tray menu.
