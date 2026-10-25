<script setup lang="ts">
/**
 * 对话面板（窗口 B「对话」Tab）：左侧会话列表 + 右侧消息流 + 底部输入区。
 *
 * 数据来源：会话列表/历史消息走 IPC（`chat_list_sessions` / `chat_get_messages`），
 * 流式增量走 `chatStore.streamingText`（窗口 A 不再显示流式回复）。
 *
 * 会话语义：`currentId` 仅内存态——面板挂载时**继续最近会话**（无会话则空白等待首条消息），
 * 重启应用即开新会话（用户决策），历史仍可从左侧列表回看。
 */

import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import ChatInput from "../../../components/pet/ChatInput.vue";
import { deleteSession, getMessages, listSessions } from "../../../api/chat";
import { useChatStream } from "../../../composables/useChatStream";
import { pushToast } from "../../../composables/useToast";
import {
  chatStore,
  removeSession,
  setMessages,
  setSessions,
  startNewSession,
  switchSession,
} from "../../../state/chatStore";

const stream = useChatStream();
const unlisteners: UnlistenFn[] = [];
const messageListEl = ref<HTMLDivElement | null>(null);
/** 联网搜索开关（默认关；经 ChatInput 受控切换，随发送透传给后端）。 */
const webSearch = ref(false);

/** 刷新会话列表（后端按最后活跃倒序）。 */
async function refreshSessions(): Promise<void> {
  try {
    setSessions(await listSessions());
  } catch (e: unknown) {
    console.warn("[chat] 会话列表加载失败", e);
  }
}

/** 切换会话：清空消息区后加载该会话历史。 */
async function selectSession(sessionId: string): Promise<void> {
  if (chatStore.currentId === sessionId) return;
  switchSession(sessionId);
  try {
    setMessages(await getMessages(sessionId));
    scrollToBottom();
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : "会话消息加载失败", "error");
  }
}

/** 删除会话（Rust 侧 messages + summaries 同事务清理）。 */
async function dropSession(sessionId: string): Promise<void> {
  try {
    await deleteSession(sessionId);
    removeSession(sessionId);
    if (chatStore.currentId === sessionId) startNewSession();
  } catch (e: unknown) {
    pushToast(e instanceof Error ? e.message : "删除会话失败", "error");
  }
}

function onSend(text: string): void {
  stream.send(text, [], webSearch.value);
  scrollToBottom();
}

function scrollToBottom(): void {
  void nextTick(() => {
    const el = messageListEl.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

// 新消息到达或流式推进时贴底（消息区可滚动，历史回看靠手动上滚）
watch(
  () => [chatStore.messages.length, chatStore.streamingText],
  () => scrollToBottom(),
);

onMounted(async (): Promise<void> => {
  await refreshSessions();
  // 继续最近会话；无历史会话则保持空白（首条消息发送时才生成 sessionId）
  const recent = chatStore.sessions[0];
  if (recent) await selectSession(recent.sessionId);
  // 一轮流结束 → 刷新列表（新会话/新消息此时才进入列表）
  unlisteners.push(
    await listen("chat:done", () => {
      void refreshSessions();
    }),
  );
});

onUnmounted((): void => {
  unlisteners.forEach((fn) => fn());
});
</script>

<template>
  <div class="chat-panel">
    <aside class="chat-panel__side">
      <button
        class="chat-new"
        type="button"
        @click="startNewSession()"
      >
        + 新建对话
      </button>
      <ul class="chat-list scroll-area">
        <li
          v-for="s in chatStore.sessions"
          :key="s.sessionId"
          class="chat-item"
          :class="{ 'is-active': s.sessionId === chatStore.currentId }"
          @click="selectSession(s.sessionId)"
        >
          <div class="chat-item__body">
            <span class="chat-item__title">{{ s.preview || "新对话" }}</span>
            <span class="chat-item__meta">{{ s.messageCount }} 条</span>
          </div>
          <button
            class="chat-item__del"
            type="button"
            aria-label="删除会话"
            @click.stop="dropSession(s.sessionId)"
          >
            ✕
          </button>
        </li>
      </ul>
      <p
        v-if="chatStore.sessions.length === 0"
        class="chat-empty-list"
      >
        还没有对话，说一句话开始吧
      </p>
    </aside>

    <section class="chat-panel__main">
      <div
        ref="messageListEl"
        class="chat-messages scroll-area"
      >
        <div
          v-for="m in chatStore.messages"
          :key="m.key"
          class="chat-msg"
          :class="m.role === 0 ? 'is-user' : 'is-pet'"
        >
          <div class="chat-msg__text">{{ m.content }}</div>
          <ul
            v-if="m.role === 1 && m.sources && m.sources.length"
            class="chat-sources"
          >
            <li
              v-for="(s, i) in m.sources"
              :key="i"
              class="chat-sources__item"
            >
              <a
                :href="s.url"
                target="_blank"
                rel="noopener noreferrer"
                class="chat-sources__link"
              >
                <svg
                  class="ico-globe"
                  viewBox="0 0 24 24"
                  width="12"
                  height="12"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <circle
                    cx="12"
                    cy="12"
                    r="9"
                  />
                  <path d="M3 12h18" />
                  <path d="M12 3c2.6 2.7 2.6 15.3 0 18M12 3c-2.6 2.7-2.6 15.3 0 18" />
                </svg>
                <span class="chat-sources__title">{{ s.title || s.url }}</span>
              </a>
            </li>
          </ul>
        </div>
        <div
          v-if="chatStore.streamingText || chatStore.isStreaming"
          class="chat-msg is-pet"
        >
          <div class="chat-msg__text">
            {{ chatStore.streamingText
            }}<span
              v-if="chatStore.isStreaming"
              class="caret"
              >▍</span
            >
          </div>
          <ul
            v-if="chatStore.streamingSources.length"
            class="chat-sources"
          >
            <li
              v-for="(s, i) in chatStore.streamingSources"
              :key="i"
              class="chat-sources__item"
            >
              <a
                :href="s.url"
                target="_blank"
                rel="noopener noreferrer"
                class="chat-sources__link"
              >
                <svg
                  class="ico-globe"
                  viewBox="0 0 24 24"
                  width="12"
                  height="12"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <circle
                    cx="12"
                    cy="12"
                    r="9"
                  />
                  <path d="M3 12h18" />
                  <path d="M12 3c2.6 2.7 2.6 15.3 0 18M12 3c-2.6 2.7-2.6 15.3 0 18" />
                </svg>
                <span class="chat-sources__title">{{ s.title || s.url }}</span>
              </a>
            </li>
          </ul>
        </div>
        <p
          v-if="!chatStore.messages.length && !chatStore.streamingText"
          class="chat-placeholder"
        >
          随便聊点什么吧～我会记住你说过的话
        </p>
      </div>
      <ChatInput
        v-model:web-search="webSearch"
        class="chat-composer"
        :disabled="chatStore.isStreaming"
        @send="onSend"
      />
    </section>
  </div>
</template>

<style scoped>
.chat-panel {
  display: grid;
  grid-template-columns: 232px 1fr;
  gap: 16px;
  height: 100%;
  min-height: 0;
}
.chat-panel__side {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  padding: 12px;
  border-radius: 16px;
  background: var(--bg-soft);
  border: 1px solid var(--line);
}
.chat-new {
  border: none;
  border-radius: var(--r-sm);
  padding: 9px 12px;
  background: var(--matcha-deep);
  color: #fff;
  font: 600 13px var(--font-ui);
  cursor: pointer;
  transition: filter var(--dur-fast) var(--ease-out);
}
.chat-new:hover {
  filter: brightness(1.06);
}
.chat-list {
  list-style: none;
  overflow-y: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.chat-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  border-radius: var(--r-sm);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out);
}
.chat-item:hover {
  background: var(--bg-window);
}
.chat-item.is-active {
  background: var(--bg-window);
  box-shadow: var(--shadow-card);
}
.chat-item__body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.chat-item__title {
  font-size: 13px;
  font-weight: 600;
  color: var(--ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chat-item__meta {
  font-size: 11px;
  color: var(--ink-soft);
}
.chat-item__del {
  border: none;
  background: none;
  color: var(--ink-soft);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 6px;
  font-size: 12px;
}
.chat-item__del:hover {
  background: var(--peach-soft);
  color: var(--peach);
}
.chat-empty-list {
  font-size: 12px;
  color: var(--ink-soft);
  padding: 4px 2px;
}
.chat-panel__main {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
}
.chat-messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  border-radius: 16px;
  background: var(--surface-panel);
  border: 1px solid var(--line);
}
.chat-msg {
  max-width: 78%;
  padding: 8px 12px;
  border-radius: 14px;
  font: 500 13px/1.55 var(--font-ui);
  white-space: pre-wrap;
  word-break: break-word;
}
.chat-msg.is-pet {
  align-self: flex-start;
  background: var(--surface-bubble);
  color: var(--ink);
  border-bottom-left-radius: 6px;
}
.chat-msg.is-user {
  align-self: flex-end;
  background: var(--matcha-deep);
  color: #fff;
  border-bottom-right-radius: 6px;
}
/* 气泡内引用来源条 */
.chat-msg__text {
  white-space: pre-wrap;
  word-break: break-word;
}
.chat-sources {
  list-style: none;
  margin: 8px 0 0;
  padding: 8px 0 0;
  border-top: 1px solid var(--border-soft);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.chat-sources__item {
  font-size: 11px;
}
.chat-sources__link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--ink-soft);
  text-decoration: none;
  transition:
    color var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
}
.ico-globe {
  flex-shrink: 0;
}
.chat-sources__link:hover {
  color: var(--ink);
  transform: translateY(-1px);
}
.chat-sources__title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 240px;
}
.caret {
  margin-left: 1px;
  animation: caret-blink 1s steps(1) infinite;
}
@keyframes caret-blink {
  50% {
    opacity: 0;
  }
}
.chat-placeholder {
  margin: auto;
  font-size: 13px;
  color: var(--ink-soft);
}
.chat-composer {
  flex-shrink: 0;
}
</style>
