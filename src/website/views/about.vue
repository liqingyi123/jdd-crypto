<script setup lang="ts">
const base = import.meta.env.BASE_URL;

const contacts = [
  { type: "QQ", value: "1787750205" },
  { type: "微信", value: "qinghe6971" },
];

const roadmap = [
  "转 KV 功能支持密文、明文两种文本格式",
  "网络代理功能，实时捕获任意应用的网络请求",
  "插件管理功能支持编辑器主题、加解密预设",
  "彩虹屁 + 节日祝福语功能",
  "将结果自动复制到剪贴板",
];
</script>

<template>
  <div class="page-section">
    <div class="container about">
      <img class="about-logo" :src="`${base}app-icon.png`" width="72" height="72" alt="多多解密" />
      <p class="section-kicker">About</p>
      <h1 class="section-title display-font">关于多多解密</h1>
      <p class="section-lead">
        奖多多内部专用跨平台桌面端加解密工具。当前支持 Windows 与 macOS（Universal），面向日常加解密与文本处理场景。
      </p>

      <div class="panels">
        <section class="panel">
          <h2>产品定位</h2>
          <p>
            用悬浮角标与托盘降低启动成本，用剪贴板询问与鼠标跟随减少复制粘贴往返，用插件槽位承载可替换的拖尾特效等扩展能力。
          </p>
        </section>
        <section class="panel">
          <h2>联系方式</h2>
          <ul>
            <li v-for="item in contacts" :key="item.type">
              <span class="label">{{ item.type }}</span>
              <span>{{ item.value }}</span>
            </li>
          </ul>
        </section>
        <section class="panel panel-wide">
          <h2>规划中</h2>
          <ol>
            <li v-for="(item, index) in roadmap" :key="index">{{ item }}</li>
          </ol>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.about {
  max-width: 880px;
}

.about-logo {
  display: block;
  width: 72px;
  height: 72px;
  margin-bottom: 16px;
  border-radius: 16px;
  object-fit: contain;
  box-shadow: var(--shadow);
}

.panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-top: 40px;
}

.panel {
  padding: 22px 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--bg-elevated);
}

.panel-wide {
  grid-column: 1 / -1;
}

.panel h2 {
  margin: 0 0 10px;
  font-size: 1.05rem;
}

.panel p,
.panel li {
  margin: 0;
  color: var(--text-muted);
}

.panel ul,
.panel ol {
  margin: 0;
  padding-left: 1.2rem;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.panel ul {
  list-style: none;
  padding-left: 0;
}

.panel ul li {
  display: flex;
  gap: 12px;
}

.label {
  min-width: 3rem;
  color: var(--brand);
  font-weight: 600;
}

@media (max-width: 640px) {
  .panels {
    grid-template-columns: 1fr;
  }
}
</style>
