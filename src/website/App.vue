<script setup lang="ts">
import SiteHeader from "./components/site-header.vue";
import GlobalTrailLayer from "./components/global-trail-layer.vue";
</script>

<template>
  <div class="site-shell">
    <GlobalTrailLayer />
    <SiteHeader />
    <main class="site-main">
      <RouterView />
    </main>
    <footer class="site-footer">
      <div class="container">© 多多解密 · 奖多多内部桌面加解密工具</div>
    </footer>
  </div>
</template>
