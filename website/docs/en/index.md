---
layout: page
---

<Hero
  emoji="🐾"
  title="Qiqi · A Digital Life on Your Desktop"
  subtitle="With personality, memory, and initiative — it talks to you first, and stays with you every day."
  primary-text="Download Now"
  primary-link="/en/download"
  secondary-text="Explore Features"
  secondary-link="/en/features"
/>

## What it does

<FeatureGrid :features="[
  { emoji: '🪟', title: 'Dual-window Pet', desc: 'A transparent, always-on-top pet layer plus a self-drawn settings panel living quietly in your screen corner.' },
  { emoji: '🍮', title: 'Jelly Physics', desc: 'Q-bouncy on click, breathing while idle, stretchy on drag — a healing, tactile feel.' },
  { emoji: '💗', title: 'Affection Growth', desc: 'Four relationship tiers from Stranger to Bond; the more you interact, the closer it gets.' },
  { emoji: '🧠', title: 'AI Chat & Memory', desc: 'Streaming chat rendered word by word, with automatic fact and summary recall. Bring your own key or use the built-in trial quota.' },
  { emoji: '⏰', title: 'Proactive Moments', desc: 'Reaches out by time windows and context, and stays quiet while you work or game.' },
  { emoji: '🐶', title: 'Switchable Avatars', desc: 'Built-in cat, dog and custom PNG avatars, swappable in one click; the preview name follows the selected avatar.' },
  { emoji: '🧲', title: 'Edge Snap & Silence', desc: 'Drag to the left or right edge to lie down against it (always on). Snap Silence only stops proactive messages — everything else continues.' },
  { emoji: '📖', title: 'Selection Translation', desc: 'Select on-screen text to translate instantly, with locally managed translation credentials.' }
]" />

## Why Qiqi

<FeatureGrid :features="[
  { emoji: '🔒', title: 'Local Encryption', desc: 'All memories and settings are encrypted locally with SQLCipher; keys never leave your machine.' },
  { emoji: '⚡', title: 'Light & Efficient', desc: 'A Rust system shell with a WebView renderer keeps idle overhead tiny.' },
  { emoji: '🛠️', title: 'Zero-friction Setup', desc: 'A built-in API wizard gets you chatting in three minutes with just a key paste.' }
]" :columns="3" />

<div class="cta-wrap">
  <a class="jelly-btn jelly-btn--primary" href="/en/download">Download for Windows — Free</a>
</div>

<style>
.cta-wrap { text-align: center; margin: 36px 0 12px; }
</style>
