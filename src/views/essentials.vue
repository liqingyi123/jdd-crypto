<script setup lang="ts">
import { computed, onMounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ElMessage, ElMessageBox } from "element-plus";
import MonacoEditor from "@/components/monaco-editor.vue";
import { useEssentialsDownload } from "@/composables/use-essentials-download";

type ToolId = "activator" | "driverMgr" | "authHelper";
type AssetId =
  | "editorconfig"
  | "eslint"
  | "gitlens"
  | "prettier"
  | "volar"
  | "vueDevtools"
  | "jddCryptoExt"
  | "tampermonkey"
  | "baiduPanHelper";
type VoltaAction = "install" | "enablePnpm" | "upgrade" | "base" | "list" | "pkg";

interface OpenOrDownloadResult {
  path: string;
  downloaded: boolean;
}

interface DailyItem {
  id: string;
  label: string;
  url: string;
}

interface AssetItem {
  id: AssetId;
  label: string;
}

interface VoltaStatus {
  installed: boolean;
  bin: string | null;
  version: string | null;
  log: string;
}

interface VoltaRunResult {
  ok: boolean;
  log: string;
  version: string | null;
}

const VPN_DOC_FALLBACK = "https://docx.jddops.com/#/doc/Kd1s3qBu";

const AV_ERROR_MARKERS = [
  "卷已被外部更改",
  "不再有效",
  "拒绝访问",
  "Access is denied",
  "access is denied",
  "被阻止",
  "无法访问",
  "找不到文件",
  "cannot find",
  "Virus",
  "quarantine",
  "打开失败",
];

const {
  activeId,
  progressPercent,
  downloadedLabel,
  totalLabel,
  totalBytes,
  end: endDownload,
  ensureProgressListener,
} = useEssentialsDownload();

const toolLoading = shallowRef<ToolId | null>(null);
const assetLoading = shallowRef<AssetId | null>(null);
const dailyOpening = shallowRef<string | null>(null);
const voltaLoading = shallowRef<VoltaAction | null>(null);
const voltaInstalled = shallowRef(false);
const voltaVersion = shallowRef("");
const voltaLog = shallowRef("");
const voltaLogVisible = shallowRef(false);
const pkgName = shallowRef("");
const pkgVersion = shallowRef("");
const openingVpn = shallowRef(false);
const cacheDir = shallowRef("");
const openingCacheDir = shallowRef(false);
const downloading = computed(() => activeId.value !== null);

const TOOL_LABELS: Record<ToolId, string> = {
  activator: "激活程序",
  driverMgr: "驱动管理程序",
  authHelper: "认证助手",
};

const dailyItems: DailyItem[] = [
  { id: "cursor", label: "Cursor", url: "https://cursor.com/cn/download" },
  { id: "trae", label: "Trae", url: "https://www.trae.cn/download" },
  {
    id: "wecom",
    label: "企业微信",
    url: "https://work.weixin.qq.com/#indexDownload",
  },
  {
    id: "whistle",
    label: "Whistle",
    url: "https://github.com/avwo/whistle-client/releases",
  },
  {
    id: "charles",
    label: "花瓶",
    url: "https://www.charles.xin/download/index.html",
  },
];

const editorAssets: AssetItem[] = [
  { id: "editorconfig", label: "EditorConfig" },
  { id: "eslint", label: "EsLint" },
  { id: "gitlens", label: "GitLens" },
  { id: "prettier", label: "Prettier" },
  { id: "volar", label: "Volar" },
];

const browserAssets: AssetItem[] = [
  { id: "vueDevtools", label: "vue-devtool" },
  { id: "jddCryptoExt", label: "多多解密" },
  { id: "tampermonkey", label: "篡改猴" },
  { id: "baiduPanHelper", label: "百度网盘文件转存助手" },
];

const versionLabel = computed(() => {
  const v = voltaVersion.value.trim();
  if (!v) {
    return "";
  }
  return v.startsWith("v") ? v : `v${v}`;
});

const downloadLabel = computed(() => {
  const id = activeId.value ?? assetLoading.value ?? toolLoading.value;
  if (!id) {
    return "";
  }
  if (id === "volta") {
    return "Volta";
  }
  if (id in TOOL_LABELS) {
    return TOOL_LABELS[id as ToolId];
  }
  const asset = [...editorAssets, ...browserAssets].find((item) => item.id === id);
  return asset?.label ?? id;
});

const progressHint = computed(() =>
  downloadLabel.value ? `正在下载${downloadLabel.value}` : "正在下载…",
);

const showOsProgress = computed(
  () =>
    downloading.value &&
    (toolLoading.value === "activator" || toolLoading.value === "driverMgr"),
);

const showNetworkProgress = computed(
  () => downloading.value && toolLoading.value === "authHelper",
);

const showVoltaProgress = computed(
  () =>
    downloading.value &&
    (voltaLoading.value === "install" || voltaLoading.value === "upgrade"),
);

const showEditorProgress = computed(
  () =>
    downloading.value &&
    assetLoading.value !== null &&
    editorAssets.some((item) => item.id === assetLoading.value),
);

const showBrowserProgress = computed(
  () =>
    downloading.value &&
    assetLoading.value !== null &&
    browserAssets.some((item) => item.id === assetLoading.value),
);

function looksLikeAvBlock(message: string): boolean {
  return AV_ERROR_MARKERS.some((marker) => message.includes(marker));
}

async function loadCacheDir() {
  try {
    cacheDir.value = await invoke<string>("essentials_cache_dir");
  } catch {
    cacheDir.value = "";
  }
}

async function openCacheDir() {
  if (openingCacheDir.value) {
    return;
  }
  openingCacheDir.value = true;
  try {
    await invoke("open_essentials_cache_dir");
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    openingCacheDir.value = false;
  }
}

async function confirmAvRiskBeforeOpen(): Promise<boolean> {
  const pathHint = cacheDir.value
    ? `\n\n缓存目录：\n${cacheDir.value}`
    : "";
  try {
    await ElMessageBox.confirm(
      `部分工具可能被杀软误报拦截。若出现「文件无效 / 卷已被更改 / 拒绝访问」等提示，请将缓存目录加入杀软信任或白名单后再重试。${pathHint}`,
      "杀软误报提示",
      {
        type: "warning",
        confirmButtonText: "继续打开",
        cancelButtonText: "打开缓存目录",
        distinguishCancelAndClose: true,
      },
    );
    return true;
  } catch (action) {
    if (action === "cancel") {
      await openCacheDir();
    }
    return false;
  }
}

async function showAvBlockHelp(errorMessage: string) {
  const pathHint = cacheDir.value
    ? `\n\n请将以下目录加入杀软信任/白名单后重试：\n${cacheDir.value}`
    : "\n\n请将装机必备缓存目录加入杀软信任/白名单后重试。";
  try {
    await ElMessageBox.confirm(
      `${errorMessage}${pathHint}`,
      "可能被杀软拦截",
      {
        type: "warning",
        confirmButtonText: "打开缓存目录",
        cancelButtonText: "关闭",
        distinguishCancelAndClose: true,
      },
    );
    await openCacheDir();
  } catch {
    // dismissed
  }
}

async function openTool(id: ToolId) {
  if (toolLoading.value || assetLoading.value || voltaLoading.value) {
    return;
  }
  const ok = await confirmAvRiskBeforeOpen();
  if (!ok) {
    return;
  }
  toolLoading.value = id;
  await ensureProgressListener();
  try {
    const result = await invoke<OpenOrDownloadResult>("open_or_download_essential", {
      toolId: id,
    });
    ElMessage.success(result.downloaded ? "下载完成并已打开" : "已打开本地文件");
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    if (!looksLikeAvBlock(msg)) {
      ElMessage.error(msg);
    }
    // 高风险工具打开失败时引导加白（杀软隔离时常表现为卷变更/拒绝访问等）
    await showAvBlockHelp(msg);
  } finally {
    endDownload();
    toolLoading.value = null;
  }
}

async function openDaily(item: DailyItem) {
  if (dailyOpening.value) {
    return;
  }
  dailyOpening.value = item.id;
  try {
    await openUrl(item.url);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    dailyOpening.value = null;
  }
}

async function downloadAsset(id: AssetId) {
  if (toolLoading.value || assetLoading.value || voltaLoading.value) {
    return;
  }
  assetLoading.value = id;
  await ensureProgressListener();
  try {
    const result = await invoke<OpenOrDownloadResult>("download_essential_asset", {
      assetId: id,
    });
    ElMessage.success(
      result.downloaded ? "下载完成，已打开所在目录" : "已打开所在目录",
    );
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    endDownload();
    assetLoading.value = null;
  }
}

async function openVpnDoc() {
  if (openingVpn.value) {
    return;
  }
  openingVpn.value = true;
  try {
    let url = VPN_DOC_FALLBACK;
    try {
      url = await invoke<string>("essentials_vpn_doc_url");
    } catch {
      // use fallback
    }
    await openUrl(url || VPN_DOC_FALLBACK);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    openingVpn.value = false;
  }
}

function setVoltaLog(text: string) {
  voltaLogVisible.value = true;
  voltaLog.value = text;
}

/** Silent status for button label / version badge; no toast. */
async function refreshVoltaStatusSilent() {
  try {
    const status = await invoke<VoltaStatus>("essentials_volta_status");
    voltaInstalled.value = status.installed;
    voltaVersion.value = status.version ?? "";
  } catch {
    // ignore
  }
}

async function runVoltaAction(
  action: Exclude<VoltaAction, "pkg">,
  command: string,
) {
  if (voltaLoading.value) {
    return;
  }
  voltaLoading.value = action;
  if (action === "install" || action === "upgrade") {
    await ensureProgressListener();
  }
  try {
    const result = await invoke<VoltaRunResult>(command);
    setVoltaLog(result.log || "");
    if (result.version) {
      voltaVersion.value = result.version;
      voltaInstalled.value = true;
    } else if (result.ok) {
      await refreshVoltaStatusSilent();
    }
    if (result.ok) {
      ElMessage.success("完成");
    } else {
      ElMessage.error("失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setVoltaLog(`# 执行失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    endDownload();
    voltaLoading.value = null;
  }
}

async function onVoltaInstallPkg() {
  const name = pkgName.value.trim();
  if (!name) {
    ElMessage.warning("请填写包名");
    return;
  }
  if (voltaLoading.value) {
    return;
  }
  voltaLoading.value = "pkg";
  try {
    const result = await invoke<VoltaRunResult>("essentials_volta_install_pkg", {
      name,
      version: pkgVersion.value.trim() || null,
    });
    setVoltaLog(result.log || "");
    if (result.version) {
      voltaVersion.value = result.version;
      voltaInstalled.value = true;
    }
    if (result.ok) {
      ElMessage.success("安装完成");
    } else {
      ElMessage.error("安装失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setVoltaLog(`# 安装失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    voltaLoading.value = null;
  }
}

onMounted(() => {
  void loadCacheDir();
  void refreshVoltaStatusSilent();
});
</script>

<template>
  <div class="page">
    <section class="section">
      <h2>操作系统</h2>
      <div class="action-row">
        <ElButton
          :loading="toolLoading === 'activator'"
          :disabled="toolLoading !== null && toolLoading !== 'activator'"
          @click="openTool('activator')"
        >
          激活程序
        </ElButton>
        <ElButton
          :loading="toolLoading === 'driverMgr'"
          :disabled="toolLoading !== null && toolLoading !== 'driverMgr'"
          @click="openTool('driverMgr')"
        >
          驱动管理程序
        </ElButton>
      </div>
      <div v-if="showOsProgress" class="download-progress">
        <p class="progress-hint">{{ progressHint }}</p>
        <ElProgress
          :percentage="totalBytes ? progressPercent : undefined"
          :indeterminate="!totalBytes"
          :stroke-width="10"
        />
        <p class="progress-meta">
          已下载 {{ downloadedLabel }}
          <template v-if="totalLabel"> / {{ totalLabel }}</template>
        </p>
      </div>
    </section>

    <section class="section">
      <h2>网络认证及VPN</h2>
      <div class="action-row">
        <ElButton
          :loading="toolLoading === 'authHelper'"
          :disabled="toolLoading !== null && toolLoading !== 'authHelper'"
          @click="openTool('authHelper')"
        >
          认证助手
        </ElButton>
        <ElButton :loading="openingVpn" @click="openVpnDoc">
          远程办公VPN安装文档
        </ElButton>
      </div>
      <div v-if="showNetworkProgress" class="download-progress">
        <p class="progress-hint">{{ progressHint }}</p>
        <ElProgress
          :percentage="totalBytes ? progressPercent : undefined"
          :indeterminate="!totalBytes"
          :stroke-width="10"
        />
        <p class="progress-meta">
          已下载 {{ downloadedLabel }}
          <template v-if="totalLabel"> / {{ totalLabel }}</template>
        </p>
      </div>
    </section>

    <section class="section volta-section">
      <div class="title-row">
        <h2>Volta 安装及部署</h2>
        <span v-if="versionLabel" class="version-badge">{{ versionLabel }}</span>
        <span v-else-if="!voltaInstalled" class="version-badge">未安装</span>
      </div>
      <div class="action-row">
        <ElButton
          v-if="!voltaInstalled"
          :loading="voltaLoading === 'install'"
          :disabled="voltaLoading !== null && voltaLoading !== 'install'"
          @click="runVoltaAction('install', 'essentials_volta_install')"
        >
          一键部署
        </ElButton>
        <ElButton
          v-else
          :loading="voltaLoading === 'enablePnpm'"
          :disabled="voltaLoading !== null && voltaLoading !== 'enablePnpm'"
          @click="runVoltaAction('enablePnpm', 'essentials_volta_enable_pnpm')"
        >
          开启pnpm特性
        </ElButton>
        <ElButton
          :loading="voltaLoading === 'upgrade'"
          :disabled="voltaLoading !== null && voltaLoading !== 'upgrade'"
          @click="runVoltaAction('upgrade', 'essentials_volta_upgrade')"
        >
          升级版本
        </ElButton>
        <ElButton
          :loading="voltaLoading === 'base'"
          :disabled="voltaLoading !== null && voltaLoading !== 'base'"
          @click="runVoltaAction('base', 'essentials_volta_install_base')"
        >
          基础依赖安装(node、pnpm、http-server、npkill)
        </ElButton>
        <ElButton
          :loading="voltaLoading === 'list'"
          :disabled="voltaLoading !== null && voltaLoading !== 'list'"
          @click="runVoltaAction('list', 'essentials_volta_list')"
        >
          查看已装依赖
        </ElButton>
      </div>
      <div class="pkg-row">
        <ElInput
          v-model="pkgName"
          clearable
          placeholder="包名，例如 python"
          class="pkg-name"
        />
        <ElInput
          v-model="pkgVersion"
          clearable
          placeholder="版本（1.0.0）（可选）"
          class="pkg-version"
        />
        <ElButton
          class="pkg-install-btn"
          type="primary"
          :loading="voltaLoading === 'pkg'"
          :disabled="voltaLoading !== null && voltaLoading !== 'pkg'"
          @click="onVoltaInstallPkg"
        >
          安装
        </ElButton>
      </div>

      <div v-if="showVoltaProgress" class="download-progress">
        <p class="progress-hint">{{ progressHint }}</p>
        <ElProgress
          :percentage="totalBytes ? progressPercent : undefined"
          :indeterminate="!totalBytes"
          :stroke-width="10"
        />
        <p class="progress-meta">
          已下载 {{ downloadedLabel }}
          <template v-if="totalLabel"> / {{ totalLabel }}</template>
        </p>
      </div>

      <template v-if="voltaLogVisible">
        <div class="log-head">
          <h3>执行日志</h3>
        </div>
        <div class="log-pane">
          <MonacoEditor
            v-model="voltaLog"
            language="shell"
            read-only
            :folding="false"
            placeholder="点击上方按钮查看执行日志"
          />
        </div>
      </template>
    </section>

    <section class="section">
      <h2>日常工作</h2>
      <div class="action-row">
        <ElButton
          v-for="item in dailyItems"
          :key="item.id"
          :loading="dailyOpening === item.id"
          :disabled="dailyOpening !== null && dailyOpening !== item.id"
          @click="openDaily(item)"
        >
          {{ item.label }}
        </ElButton>
      </div>
    </section>

    <section class="section">
      <h2>编辑器插件</h2>
      <div class="action-row">
        <ElButton
          v-for="item in editorAssets"
          :key="item.id"
          :loading="assetLoading === item.id"
          :disabled="
            (assetLoading !== null && assetLoading !== item.id) ||
            toolLoading !== null ||
            voltaLoading !== null
          "
          @click="downloadAsset(item.id)"
        >
          {{ item.label }}
        </ElButton>
      </div>
      <div v-if="showEditorProgress" class="download-progress">
        <p class="progress-hint">{{ progressHint }}</p>
        <ElProgress
          :percentage="totalBytes ? progressPercent : undefined"
          :indeterminate="!totalBytes"
          :stroke-width="10"
        />
        <p class="progress-meta">
          已下载 {{ downloadedLabel }}
          <template v-if="totalLabel"> / {{ totalLabel }}</template>
        </p>
      </div>
    </section>

    <section class="section">
      <h2>浏览器插件</h2>
      <div class="action-row">
        <ElButton
          v-for="item in browserAssets"
          :key="item.id"
          :loading="assetLoading === item.id"
          :disabled="
            (assetLoading !== null && assetLoading !== item.id) ||
            toolLoading !== null ||
            voltaLoading !== null
          "
          @click="downloadAsset(item.id)"
        >
          {{ item.label }}
        </ElButton>
      </div>
      <div v-if="showBrowserProgress" class="download-progress">
        <p class="progress-hint">{{ progressHint }}</p>
        <ElProgress
          :percentage="totalBytes ? progressPercent : undefined"
          :indeterminate="!totalBytes"
          :stroke-width="10"
        />
        <p class="progress-meta">
          已下载 {{ downloadedLabel }}
          <template v-if="totalLabel"> / {{ totalLabel }}</template>
        </p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  overflow: auto;
}

.section {
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elevated);
}

.volta-section {
  display: flex;
  flex-direction: column;
}

.title-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 12px;
}

h2 {
  margin: 0 0 12px;
  font-size: 15px;
}

.title-row h2 {
  margin-bottom: 0;
}

.version-badge {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-muted);
}

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.pkg-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
  align-items: center;
}

.pkg-name,
.pkg-version {
  flex: 1 1 0;
  min-width: 0;
}

.pkg-install-btn {
  flex: 0 0 96px;
}

.download-progress {
  margin-top: 12px;
}

.progress-hint,
.progress-meta {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--text-muted);
}

.progress-meta {
  margin-top: 8px;
  margin-bottom: 0;
}

.log-head {
  margin: 14px 0 8px;
}

.log-head h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.log-pane {
  height: 180px;
  max-height: 180px;
  min-height: 180px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

</style>
