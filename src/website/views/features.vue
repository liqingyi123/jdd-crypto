<script setup lang="ts">
import { FEATURE_ITEMS } from "../data/features";
import TrailDemo from "../components/trail-demo.vue";
import { detectPlatform } from "../composables/use-website-download";

const isMacos = detectPlatform() === "macos";
</script>

<template>
  <div class="page-section">
    <div class="container">
      <p class="section-kicker">Features</p>
      <h1 class="section-title display-font">功能介绍</h1>
      <p class="section-lead">
        围绕加解密日常场景打磨的桌面能力：从角标、剪贴板气泡到文本对比，均可按需开关与自定义快捷键。
      </p>
      <div class="feature-grid">
        <article v-for="item in FEATURE_ITEMS" :key="item.id" class="feature-card">
          <h2>{{ item.title }}</h2>
          <p>{{ item.summary }}</p>
        </article>
      </div>

      <section class="trail-block">
        <div class="trail-copy">
          <p class="section-kicker">Live demo</p>
          <h2 class="section-title display-font">鼠标拖尾特效</h2>
          <p class="section-lead">
            与桌面端同一套特效引擎，已在全站挂载。可在此切换特效，并为绚丽流星 / 连线点阵 / 心动回忆调整颜色。
          </p>
        </div>
        <TrailDemo />
      </section>
      <aside v-if="isMacos" class="macos-tips" aria-label="macOS 使用提示">
        <p class="macos-tips-title">macOS 使用提示</p>
        <ol>
          <li>
            鼠标跟随（macOS）与 Windows 相同，默认快捷键为
            <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>G</kbd>（Mac 键盘上的 Control 即 Ctrl）；文本对比模式默认为
            <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd>，均可在设置中自定义。首次使用若快捷键无反应、或选中文本后无法自动复制，请在
            <strong>系统设置 → 隐私与安全性 → 辅助功能</strong>
            中为「多多解密」开启权限（全局快捷键与模拟 ⌘C 均依赖此项）。
          </li>
          <li>
            内网检查更新，发现新版本后，Mac 下载
            <code>多多解密_{version}_universal.dmg</code>，打开 dmg 后手动拖入「应用程序」；未签名包若被 Gatekeeper 拦截，请右键应用选择「打开」。
          </li>
        </ol>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.macos-tips {
  margin-top: 28px;
  padding: 18px 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--bg-elevated);
  box-shadow: var(--shadow);
}

.macos-tips-title {
  margin: 0 0 12px;
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--text);
}

.macos-tips ol {
  margin: 0;
  padding-left: 1.25rem;
  color: var(--text-muted);
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 0.95rem;
  line-height: 1.65;
}

.macos-tips kbd,
.macos-tips code {
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 0.88em;
}

.macos-tips kbd {
  padding: 2px 6px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-muted);
  color: var(--text);
}

.macos-tips code {
  padding: 2px 6px;
  border-radius: 6px;
  background: var(--bg-muted);
  color: var(--text);
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
  margin: 40px 0 72px;
}

.feature-card {
  padding: 22px 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--bg-elevated);
  box-shadow: var(--shadow);
}

.feature-card h2 {
  margin: 0 0 8px;
  font-size: 1.1rem;
}

.feature-card p {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.95rem;
}

.trail-block {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.trail-copy {
  max-width: 40rem;
}
</style>
