# Changelog

## v0.1.1 · 2026-11-16

- 🧲 Edge snapping is now a system default: drag to the left or right edge to lie down against it, regardless of any switch.
- 🤫 "Edge-snap Sleep" is now "Snap Silence": when enabled it only stops proactive messages; chat, petting, background diary and affection decay continue as usual.
- 🎚️ Snap reveal ratio is fully independent of Snap Silence — adjustable from 30% to 80% at any time, taking effect on the next snap.
- 🐶 Custom PNG avatars can be imported, renamed and deleted; the preview name follows the selected avatar.
- 📖 Selection translation is here, with chat and translation using separate credentials managed on the Rust side.
- 🧠 Chat reliability: long SSE streams are no longer cut off by a 30-second timeout, `[DONE]` now ends streams cleanly, per-frame usage no longer swallows text deltas, and HTTP/1.1 is enforced to avoid decode errors; built-in quota eligibility was corrected.
- 🩹 Fixed the first-snap "avatar pops out and whole window clicks through" issue with live viewport sizing, immediate hit-region reporting with out-of-bounds fallback, and a first-snap rebuild fallback.

## v0.1.0 · 2026-11-05

- 🎉 First public release.
- 🪟 Dual-window pet: transparent always-on-top pet layer + self-drawn settings panel.
- 🍮 Jelly physics: click rebound, breathing idle, drag stretch.
- 💗 Affection engine: four tiers with emotional expression.
- 🧠 AI chat & memory recall: streaming chat + local vector memory.
- ⏰ Proactive moments: time windows + context awareness.
- 🐶 Built-in cat / dog avatar switching.
- 🔍 Web search enrichment.
- 🔒 Fully local SQLCipher encrypted storage.
