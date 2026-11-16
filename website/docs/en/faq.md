# FAQ

### Is Qiqi free?

Qiqi itself is completely free. AI chat can use the built-in trial quota (Zhipu / SiliconFlow), or you can enter your own OpenAI-compatible API key in settings (BYOK). Both support streaming chat and local memory recall.

### How do edge snapping and "Snap Silence" relate?

Edge snapping is a system default: drag Qiqi to the left or right edge of your screen and it lies down against it, revealing the configured portion of the avatar; drag it away to restore. No switch controls this.

The "Snap Silence" switch **only stops proactive messages**: while enabled, Qiqi won't initiate chats when snapped. Double-click chat, petting, background diary and affection decay all continue as usual.

### Can I adjust how much is revealed when snapped?

Yes. Settings → Avatar → "Snap Reveal Ratio" adjusts it between 30% and 80%, saved as soon as you release the slider. It is independent of "Snap Silence": you can change it with the switch on or off, and it takes effect the next time you snap.

### Is my chat history and memory safe?

Very safe. All messages, facts, summaries, and settings are stored in a local SQLite database encrypted with SQLCipher. The key is kept in your system credentials; data is **never uploaded to any cloud**. Neither API keys nor built-in service credentials are written to the frontend or stored in plaintext.

### Does Qiqi keep consuming resources?

No. Built on a Rust system shell with a WebView renderer, it has minimal CPU usage when idle and drops to a lower frame rate to save power.

### Why can't I see the pet window?

The pet window is transparent and always on top in a screen corner, and may be covered by other windows. Try moving the foreground window, or adjust its position and visibility in the settings panel.

### Does it support macOS / Linux?

Currently only Windows 10 and later (x64) are supported. Other platforms are on the roadmap.

### How do I switch cat / dog / custom avatars?

Open Settings → Avatar to switch between the built-in cat, dog and any imported custom PNG avatars. You can also rename or delete custom avatars. The name under the preview follows the currently selected avatar.

### Does selection translation need extra setup?

No. Select text on screen and use the toolbar to translate it. Translation credentials are built in and managed on the Rust side; chat and translation use separate credential chains.

### How do I quit the app?

Qiqi stays resident in the background; closing the settings window only hides it. The real exit entry is in the system tray menu.
