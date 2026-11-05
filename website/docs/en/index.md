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
  { emoji: '🧠', title: 'AI Chat & Memory', desc: 'OpenAI-compatible streaming chat that quietly recalls what you have told it.' },
  { emoji: '⏰', title: 'Proactive Moments', desc: 'Reaches out by time windows and context, and stays quiet while you work or game.' },
  { emoji: '🐶', title: 'Switchable Avatars', desc: 'Built-in cat and dog avatars, swappable; custom avatars planned ahead.' }
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
