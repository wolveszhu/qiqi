---
layout: page
---

<Hero
  emoji="🐾"
  title="七七 · 住在你桌面上的数字生命"
  subtitle="有性格、有记忆、有主动性。它会主动找你说话，陪你度过每一天。"
  primary-text="立即下载"
  primary-link="/download"
  secondary-text="了解功能"
  secondary-link="/features"
/>

## 它会做什么

<FeatureGrid :features="[
  { emoji: '🪟', title: '双窗口桌宠', desc: '透明置顶的桌宠层 + 自绘设置面板，安静地住在你屏幕角落。' },
  { emoji: '🍮', title: '果冻物理交互', desc: '点击会 Q 弹回弹，呼吸般起伏，拖拽有拉伸形变，手感治愈。' },
  { emoji: '💗', title: '好感度进化', desc: '从陌生到羁绊四级关系，互动越多越亲近，从不播报冰冷数值。' },
  { emoji: '🧠', title: 'AI 对话与记忆', desc: '流式对话逐字呈现，自动抽取事实与摘要；支持自备 Key 或内置体验额度。' },
  { emoji: '⏰', title: '主动互动', desc: '按时间窗口与情境感知主动找你，工作与游戏时懂得安静。' },
  { emoji: '🐶', title: '形象自由切换', desc: '内置猫狗与自定义 PNG 形象一键切换，预览名称跟随当前形象。' },
  { emoji: '🧲', title: '边缘吸附 · 贴边静默', desc: '拖到左右边缘即贴边躺倒（系统默认）；「贴边静默」只停止主动消息，其他互动照常。' },
  { emoji: '📖', title: '划词翻译', desc: '选中文字即可快速翻译，全程不改动你的剪贴板；翻译凭据由本地加密管理。' }
]" />

## 为什么选择七七

<FeatureGrid :features="[
  { emoji: '🔒', title: '本地加密', desc: '所有记忆与设置经 SQLCipher 本地加密，密钥与数据不出本机。' },
  { emoji: '⚡', title: '轻盈省电', desc: 'Rust 系统外壳 + WebView 渲染，待机低占用，几乎不打扰你。' },
  { emoji: '🛠️', title: '零门槛配置', desc: '自带 API 配置向导，粘贴 Key 即可对话，新手三分钟上手。' }
]" :columns="3" />

<div class="cta-wrap">
  <a class="jelly-btn jelly-btn--primary" href="/download">免费下载 for Windows</a>
</div>

<style>
.cta-wrap { text-align: center; margin: 36px 0 12px; }
</style>
