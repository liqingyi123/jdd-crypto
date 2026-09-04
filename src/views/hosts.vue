<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage, ElMessageBox } from "element-plus";
import MonacoEditor from "@/components/monaco-editor.vue";

type HostNature = "keep" | "exclusive";
type HostSchemeType = "local" | "remote";

const SYSTEM_ID = "__system__";

interface HostsScheme {
  id: string;
  title: string;
  content: string;
  enabled: boolean;
  source: string;
  nature: HostNature;
  readonly: boolean;
  /** App IPC: schemeType; SwitchHosts import source field: type */
  type?: HostSchemeType;
  schemeType?: HostSchemeType;
  url: string;
  refresh_interval?: number;
  refreshInterval?: number;
  last_refresh?: string;
  lastRefresh?: string;
  last_refresh_ms?: number;
  lastRefreshMs?: number;
}

function normalizeScheme(raw: HostsScheme): HostsScheme {
  const url = (raw.url ?? "").trim();
  const rawType = String(raw.schemeType ?? raw.type ?? "").toLowerCase();
  const type: HostSchemeType =
    rawType === "remote" || url.length > 0 ? "remote" : "local";
  const refresh_interval = Number(
    raw.refreshInterval ?? raw.refresh_interval ?? 0,
  );
  const last_refresh = String(raw.lastRefresh ?? raw.last_refresh ?? "");
  const last_refresh_ms = Number(
    raw.lastRefreshMs ?? raw.last_refresh_ms ?? 0,
  );
  return {
    ...raw,
    type,
    schemeType: type,
    url,
    refresh_interval: Number.isFinite(refresh_interval) ? refresh_interval : 0,
    refreshInterval: Number.isFinite(refresh_interval) ? refresh_interval : 0,
    last_refresh,
    lastRefresh: last_refresh,
    last_refresh_ms: Number.isFinite(last_refresh_ms) ? last_refresh_ms : 0,
    lastRefreshMs: Number.isFinite(last_refresh_ms) ? last_refresh_ms : 0,
    readonly: type === "remote" ? true : !!raw.readonly,
  };
}

function normalizeSchemes(list: HostsScheme[] | null | undefined): HostsScheme[] {
  if (!Array.isArray(list)) {
    return [];
  }
  return list.map((item) => normalizeScheme(item));
}

const REFRESH_OPTIONS: Array<{ label: string; value: number }> = [
  { label: "永不", value: 0 },
  { label: "5分钟", value: 300 },
  { label: "10分钟", value: 600 },
  { label: "30分钟", value: 1800 },
  { label: "1小时", value: 3600 },
  { label: "2小时", value: 7200 },
  { label: "4小时", value: 14400 },
  { label: "6小时", value: 21600 },
  { label: "12小时", value: 43200 },
  { label: "24小时", value: 86400 },
];

interface ImportResult {
  imported: number;
  skipped: number;
  conflicts?: number;
  schemes: HostsScheme[];
}

const loading = shallowRef(false);
const saving = shallowRef(false);
const importing = shallowRef(false);
const exporting = shallowRef(false);
const pulling = shallowRef(false);
const resetting = shallowRef(false);
const schemes = ref<HostsScheme[]>([]);
const selectedId = shallowRef<string | null>(null);
const draftTitle = shallowRef("");
const draftContent = shallowRef("");
const dirty = shallowRef(false);
const syncing = shallowRef(false);
const fileInput = ref<HTMLInputElement | null>(null);
const renamingId = shallowRef<string | null>(null);
const renameDraft = shallowRef("");
const createDialogVisible = shallowRef(false);
const createNature = shallowRef<HostNature>("exclusive");
const createType = shallowRef<HostSchemeType>("local");
const createUrl = shallowRef("");
const createInterval = shallowRef(0);
const deleteDialogVisible = shallowRef(false);
const deleteTarget = shallowRef<HostsScheme | null>(null);
const deleting = shallowRef(false);
const openingSystem = shallowRef(false);
const hostsWritable = shallowRef(false);
const sessionPermissionOk = shallowRef(false);
const permissionBreathing = shallowRef(false);
const requestingPermission = shallowRef(false);
const refreshingRemote = shallowRef(false);
const draftUrl = shallowRef("");
const draftInterval = shallowRef(0);
const draftType = shallowRef<HostSchemeType>("local");
const renameInputRef = ref<{ focus?: () => void; input?: HTMLInputElement } | null>(
  null,
);

let stopRenameOutsideListen: (() => void) | undefined;

const selected = computed(
  () => schemes.value.find((s) => s.id === selectedId.value) ?? null,
);

const viewingSystem = computed(() => selectedId.value === SYSTEM_ID);

const isRemoteSelected = computed(
  () =>
    !viewingSystem.value &&
    selected.value !== null &&
    isRemoteScheme(selected.value),
);

const contentReadOnly = computed(
  () =>
    viewingSystem.value ||
    draftType.value === "remote" ||
    selected.value?.readonly === true,
);

const metaEditable = computed(() => !viewingSystem.value && selected.value !== null);

const hasEditorTarget = computed(
  () => viewingSystem.value || selected.value !== null,
);

const showPermissionButton = computed(
  () => !hostsWritable.value && !sessionPermissionOk.value,
);

function isPermissionError(err: unknown): boolean {
  const text = String(err);
  return /权限|提权|管理员|UAC|Permission|Access is denied|拒绝|被取消/i.test(
    text,
  );
}

async function refreshHostsPermission() {
  try {
    hostsWritable.value = await invoke<boolean>("hosts_has_write_access");
    if (hostsWritable.value) {
      sessionPermissionOk.value = true;
      permissionBreathing.value = false;
    }
  } catch {
    hostsWritable.value = false;
  }
}

async function requestHostsPermission() {
  requestingPermission.value = true;
  try {
    await invoke("hosts_request_permission");
    await refreshHostsPermission();
    sessionPermissionOk.value = true;
    permissionBreathing.value = false;
    ElMessage.success("已获得 Host 写入权限");
    await reloadSystemIfNeeded();
  } catch (err) {
    sessionPermissionOk.value = false;
    permissionBreathing.value = true;
    ElMessage.error(`获取 Host 权限失败：${String(err)}`);
  } finally {
    requestingPermission.value = false;
  }
}

function markPermissionFailure(err: unknown) {
  if (!isPermissionError(err)) {
    return;
  }
  sessionPermissionOk.value = false;
  permissionBreathing.value = true;
}

function natureLabel(nature: string): string {
  return nature === "keep" ? "保留" : "单开";
}

function sourceLabel(source: string): string {
  return source === "imported" ? "导入" : "本地";
}

function isRemoteScheme(scheme: HostsScheme): boolean {
  const normalized = normalizeScheme(scheme);
  return normalized.type === "remote";
}

function schemeTypeOf(scheme: HostsScheme): HostSchemeType {
  return isRemoteScheme(scheme) ? "remote" : "local";
}

function schemeRefreshInterval(scheme: HostsScheme): number {
  return normalizeScheme(scheme).refresh_interval ?? 0;
}

function schemeUrl(scheme: HostsScheme): string {
  return normalizeScheme(scheme).url ?? "";
}

function schemeLastRefresh(scheme: HostsScheme): string {
  return normalizeScheme(scheme).last_refresh ?? "";
}

function cancelRename() {
  renamingId.value = null;
  renameDraft.value = "";
}

function bindRenameOutsideClose() {
  stopRenameOutsideListen?.();
  const onPointerDown = (event: PointerEvent) => {
    const target = event.target;
    if (!(target instanceof Element)) {
      cancelRename();
      return;
    }
    if (target.closest(".rename-row")) {
      return;
    }
    cancelRename();
  };
  document.addEventListener("pointerdown", onPointerDown, true);
  stopRenameOutsideListen = () => {
    document.removeEventListener("pointerdown", onPointerDown, true);
    stopRenameOutsideListen = undefined;
  };
}

watch(renamingId, async (id) => {
  if (!id) {
    stopRenameOutsideListen?.();
    return;
  }
  bindRenameOutsideClose();
  await nextTick();
  renameInputRef.value?.focus?.();
  renameInputRef.value?.input?.focus?.();
});

onBeforeUnmount(() => {
  stopRenameOutsideListen?.();
});

async function selectScheme(scheme: HostsScheme) {
  if (renamingId.value) {
    cancelRename();
  }
  if (
    dirty.value &&
    selectedId.value &&
    selectedId.value !== scheme.id &&
    selectedId.value !== SYSTEM_ID
  ) {
    ElMessage.warning("请先保存或放弃当前编辑");
    return;
  }
  selectedId.value = scheme.id;
  syncing.value = true;
  draftTitle.value = scheme.title;
  draftContent.value = scheme.content;
  draftType.value = schemeTypeOf(scheme);
  draftUrl.value = schemeUrl(scheme);
  draftInterval.value = schemeRefreshInterval(scheme);
  dirty.value = false;
  await nextTick();
  syncing.value = false;
}

async function selectSystemHosts() {
  if (renamingId.value) {
    cancelRename();
  }
  if (dirty.value && selectedId.value && selectedId.value !== SYSTEM_ID) {
    ElMessage.warning("请先保存或放弃当前编辑");
    return;
  }
  selectedId.value = SYSTEM_ID;
  syncing.value = true;
  draftTitle.value = "当前系统生效";
  dirty.value = false;
  try {
    draftContent.value = await invoke<string>("hosts_read_system");
  } catch (err) {
    draftContent.value = "";
    ElMessage.error(`读取系统 hosts 失败：${String(err)}`);
  }
  await nextTick();
  syncing.value = false;
}

async function reloadSystemIfNeeded() {
  if (!viewingSystem.value) {
    return;
  }
  try {
    draftContent.value = await invoke<string>("hosts_read_system");
  } catch {
    // keep previous content
  }
}

async function openSystemHostsFile() {
  openingSystem.value = true;
  try {
    await invoke("hosts_open_system");
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    openingSystem.value = false;
  }
}

function markDirty() {
  if (syncing.value || viewingSystem.value) {
    return;
  }
  dirty.value = true;
}

async function refresh() {
  loading.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_list");
    schemes.value = normalizeSchemes(list);
    if (viewingSystem.value) {
      await reloadSystemIfNeeded();
    } else if (
      selectedId.value &&
      !schemes.value.some((s) => s.id === selectedId.value)
    ) {
      selectedId.value = null;
      draftTitle.value = "";
      draftContent.value = "";
      dirty.value = false;
      await selectSystemHosts();
    } else if (selectedId.value) {
      const current = schemes.value.find((s) => s.id === selectedId.value);
      if (current && !dirty.value) {
        await selectScheme(current);
      }
    } else {
      await selectSystemHosts();
    }
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    loading.value = false;
  }
}

function openCreateDialog() {
  if (dirty.value) {
    ElMessage.warning("请先保存或放弃当前编辑");
    return;
  }
  if (renamingId.value) {
    cancelRename();
  }
  createNature.value = "exclusive";
  createType.value = "local";
  createUrl.value = "";
  createInterval.value = 0;
  createDialogVisible.value = true;
}

async function confirmCreate() {
  if (createType.value === "remote" && !createUrl.value.trim()) {
    ElMessage.warning("远程方案必须填写 URL");
    return;
  }
  saving.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_upsert", {
      id: null,
      title: createType.value === "remote" ? "新远程方案" : "新方案",
      content: "",
      enabled: false,
      nature: createNature.value,
      schemeType: createType.value,
      url: createType.value === "remote" ? createUrl.value.trim() : "",
      refreshInterval: createType.value === "remote" ? createInterval.value : 0,
    });
    schemes.value = normalizeSchemes(list);
    createDialogVisible.value = false;
    const created = schemes.value[schemes.value.length - 1];
    if (created) {
      await selectScheme(created);
    }
    ElMessage.success(
      createType.value === "remote" ? "已新建并刷新远程方案" : "已新建方案",
    );
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    saving.value = false;
  }
}

async function saveScheme() {
  if (!selectedId.value || viewingSystem.value) {
    return;
  }
  const title = draftTitle.value.trim();
  if (!title) {
    ElMessage.warning("标题不能为空");
    return;
  }
  if (draftType.value === "remote" && !draftUrl.value.trim()) {
    ElMessage.warning("远程方案必须填写 URL");
    return;
  }
  saving.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_upsert", {
      id: selectedId.value,
      title,
      content: draftContent.value,
      enabled: null,
      nature: null,
      schemeType: draftType.value,
      url: draftType.value === "remote" ? draftUrl.value.trim() : "",
      refreshInterval: draftType.value === "remote" ? draftInterval.value : 0,
    });
    schemes.value = normalizeSchemes(list);
    dirty.value = false;
    const current = schemes.value.find((s) => s.id === selectedId.value);
    if (current) {
      await selectScheme(current);
    }
    ElMessage.success("已保存");
    if (current?.enabled) {
      sessionPermissionOk.value = true;
      permissionBreathing.value = false;
      await refreshHostsPermission();
      await reloadSystemIfNeeded();
    }
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(String(err));
  } finally {
    saving.value = false;
  }
}

async function refreshRemoteNow() {
  if (!selectedId.value || !isRemoteSelected.value) {
    return;
  }
  refreshingRemote.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_refresh", {
      id: selectedId.value,
    });
    schemes.value = normalizeSchemes(list);
    const current = schemes.value.find((s) => s.id === selectedId.value);
    if (current) {
      await selectScheme(current);
    }
    ElMessage.success("远程 Host 已刷新");
    await reloadSystemIfNeeded();
  } catch (err) {
    ElMessage.error(`刷新失败：${String(err)}`);
  } finally {
    refreshingRemote.value = false;
  }
}

async function discardEdits() {
  if (!selected.value) {
    return;
  }
  syncing.value = true;
  draftTitle.value = selected.value.title;
  draftContent.value = selected.value.content;
  draftType.value = schemeTypeOf(selected.value);
  draftUrl.value = schemeUrl(selected.value);
  draftInterval.value = schemeRefreshInterval(selected.value);
  dirty.value = false;
  await nextTick();
  syncing.value = false;
}

async function toggleEnabled(scheme: HostsScheme, enabled: boolean) {
  try {
    const list = await invoke<HostsScheme[]>("hosts_set_enabled", {
      id: scheme.id,
      enabled,
    });
    schemes.value = normalizeSchemes(list);
    sessionPermissionOk.value = true;
    permissionBreathing.value = false;
    ElMessage.success(
      enabled
        ? `Host 切换成功：已启用「${scheme.title}」`
        : `Host 切换成功：已关闭「${scheme.title}」`,
    );
    await refreshHostsPermission();
    await reloadSystemIfNeeded();
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(`Host 切换失败：${String(err)}`);
    await refresh();
  }
}

function startRename(scheme: HostsScheme) {
  if (dirty.value) {
    ElMessage.warning("请先保存或放弃当前编辑");
    return;
  }
  renamingId.value = scheme.id;
  renameDraft.value = scheme.title;
}

async function confirmRename(scheme: HostsScheme) {
  const title = renameDraft.value.trim();
  if (!title) {
    ElMessage.warning("标题不能为空");
    return;
  }
  try {
    const list = await invoke<HostsScheme[]>("hosts_rename", {
      id: scheme.id,
      title,
    });
    schemes.value = normalizeSchemes(list);
    if (selectedId.value === scheme.id) {
      draftTitle.value = title;
    }
    cancelRename();
    ElMessage.success("已重命名");
  } catch (err) {
    ElMessage.error(String(err));
  }
}

async function changeNature(scheme: HostsScheme, nature: HostNature) {
  if (scheme.nature === nature) {
    return;
  }
  try {
    const list = await invoke<HostsScheme[]>("hosts_set_nature", {
      id: scheme.id,
      nature,
    });
    schemes.value = normalizeSchemes(list);
    ElMessage.success(`已切换为${natureLabel(nature)}`);
    if (scheme.enabled) {
      sessionPermissionOk.value = true;
      permissionBreathing.value = false;
      await refreshHostsPermission();
    }
    await reloadSystemIfNeeded();
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(String(err));
    await refresh();
  }
}

function askDeleteScheme(scheme: HostsScheme) {
  deleteTarget.value = scheme;
  deleteDialogVisible.value = true;
}

async function confirmDeleteScheme() {
  const scheme = deleteTarget.value;
  if (!scheme) {
    return;
  }
  deleting.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_delete", { id: scheme.id });
    schemes.value = normalizeSchemes(list);
    deleteDialogVisible.value = false;
    deleteTarget.value = null;
    if (renamingId.value === scheme.id) {
      cancelRename();
    }
    if (selectedId.value === scheme.id) {
      selectedId.value = null;
      draftTitle.value = "";
      draftContent.value = "";
      dirty.value = false;
      if (list.length > 0) {
        await selectScheme(list[0]);
      }
    }
    ElMessage.success("已删除");
    if (scheme.enabled) {
      sessionPermissionOk.value = true;
      permissionBreathing.value = false;
      await refreshHostsPermission();
    }
    await reloadSystemIfNeeded();
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(String(err));
  } finally {
    deleting.value = false;
  }
}

function onMoreCommand(scheme: HostsScheme, command: string) {
  if (command === "rename") {
    startRename(scheme);
    return;
  }
  if (command === "keep") {
    void changeNature(scheme, "keep");
    return;
  }
  if (command === "exclusive") {
    void changeNature(scheme, "exclusive");
    return;
  }
  if (command === "delete") {
    askDeleteScheme(scheme);
  }
}

function triggerImport() {
  fileInput.value?.click();
}

async function onFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) {
    return;
  }
  importing.value = true;
  try {
    const raw = await file.text();
    const result = await invoke<ImportResult>("hosts_import_switchhosts", {
      raw,
    });
    schemes.value = normalizeSchemes(result.schemes);
    if (!selectedId.value && schemes.value.length > 0) {
      await selectScheme(schemes.value[schemes.value.length - 1]);
    }
    ElMessage.success(
      `导入完成：成功 ${result.imported} 个，跳过 ${result.skipped} 个`,
    );
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    importing.value = false;
  }
}

async function pullPresetConfig() {
  pulling.value = true;
  try {
    const result = await invoke<ImportResult>("hosts_pull_preset");
    schemes.value = normalizeSchemes(result.schemes);
    const conflicts = Number(result.conflicts ?? 0);
    const conflictScheme = schemes.value.find((s) =>
      (s.content ?? "").includes("# >>> 本地 start"),
    );
    if (conflictScheme) {
      await selectScheme(conflictScheme);
    } else if (!selectedId.value && schemes.value.length > 0) {
      await selectScheme(schemes.value[0]);
    }
    if (conflicts > 0) {
      ElMessage.warning(
        `拉取完成：更新 ${result.imported} 个，跳过 ${result.skipped} 个；有 ${conflicts} 处同域名冲突。请查看「# >>> 本地 / 服务器」块，自行决定保留后保存`,
      );
    } else {
      ElMessage.success(
        `拉取完成：更新 ${result.imported} 个，跳过 ${result.skipped} 个`,
      );
    }
    await reloadSystemIfNeeded();
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(String(err));
  } finally {
    pulling.value = false;
  }
}

function downloadJsonFile(content: string, filename: string) {
  const blob = new Blob([content], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

async function exportSwitchhosts() {
  exporting.value = true;
  try {
    const raw = await invoke<string>("hosts_export_switchhosts");
    const stamp = new Date().toISOString().slice(0, 10);
    downloadJsonFile(raw, `switchhosts-backup-${stamp}.json`);
    ElMessage.success("导出完成");
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    exporting.value = false;
  }
}

async function resetSystemHosts() {
  try {
    await ElMessageBox.confirm(
      "将关闭全部方案开关，并清除本应用写入系统 hosts 的托管内容。是否继续？",
      "重置 Host",
      {
        type: "warning",
        confirmButtonText: "重置",
        cancelButtonText: "取消",
      },
    );
  } catch {
    return;
  }
  resetting.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_reset_system");
    schemes.value = normalizeSchemes(list);
    sessionPermissionOk.value = true;
    permissionBreathing.value = false;
    await refreshHostsPermission();
    await reloadSystemIfNeeded();
    ElMessage.success("已重置系统 Host");
  } catch (err) {
    markPermissionFailure(err);
    ElMessage.error(String(err));
  } finally {
    resetting.value = false;
  }
}

onMounted(() => {
  void refresh();
  void refreshHostsPermission();
});
</script>

<template>
  <div v-loading="loading" class="page">
    <div class="toolbar">
      <div class="toolbar-left">
        <!-- 暂时关闭新建 / 导入 / 导出，改走内网固定预置拉取
        <ElButton type="primary" :loading="saving" @click="openCreateDialog">
          新建
        </ElButton>
        <ElButton :loading="importing" @click="triggerImport">
          导入
        </ElButton>
        <ElButton :loading="exporting" @click="exportSwitchhosts">
          导出
        </ElButton>
        -->
        <ElButton type="primary" :loading="pulling" @click="pullPresetConfig">
          拉取最新配置
        </ElButton>
        <ElButton
          type="warning"
          plain
          :loading="resetting"
          @click="resetSystemHosts"
        >
          重置
        </ElButton>
        <ElButton
          v-if="showPermissionButton"
          class="perm-btn"
          :class="{ 'perm-breathe': permissionBreathing }"
          type="danger"
          plain
          :loading="requestingPermission"
          @click="requestHostsPermission"
        >
          Host应用失败？点击获取权限
        </ElButton>
        <input
          ref="fileInput"
          class="file-input"
          type="file"
          accept=".json,application/json"
          @change="onFileSelected"
        />
      </div>
    </div>

    <div class="body">
      <aside class="list">
        <div
          class="item item-system"
          :class="{ active: viewingSystem }"
          @click="selectSystemHosts"
        >
          <div class="item-main">
            <div class="item-title">当前系统生效</div>
            <div class="item-meta">
              <span class="meta-text">系统 hosts</span>
              <ElTag size="small" effect="plain" type="info">只读</ElTag>
            </div>
          </div>
          <div class="item-actions" @click.stop>
            <ElButton
              size="small"
              :loading="openingSystem"
              @click="openSystemHostsFile"
            >
              打开文件
            </ElButton>
          </div>
        </div>

        <div
          v-for="scheme in schemes"
          :key="scheme.id"
          class="item"
          :class="{ active: scheme.id === selectedId }"
          @click="selectScheme(scheme)"
        >
          <div class="item-main">
            <div
              v-if="renamingId === scheme.id"
              class="rename-row"
              @click.stop
            >
              <ElInput
                ref="renameInputRef"
                v-model="renameDraft"
                size="small"
                maxlength="64"
                @keyup.enter="confirmRename(scheme)"
                @keyup.esc="cancelRename"
              />
              <ElButton
                type="primary"
                size="small"
                class="rename-ok"
                @pointerdown.stop
                @click="confirmRename(scheme)"
              >
                ✔
              </ElButton>
            </div>
            <div v-else class="item-title" :title="scheme.title">
              {{ scheme.title }}
            </div>
            <div class="item-meta">
              <ElTag size="small" effect="plain">
                {{ sourceLabel(scheme.source) }}
              </ElTag>
              <ElTag
                v-if="isRemoteScheme(scheme)"
                size="small"
                effect="plain"
                type="danger"
              >
                远程
              </ElTag>
              <ElTag
                size="small"
                effect="plain"
                :type="scheme.nature === 'exclusive' ? 'warning' : 'success'"
              >
                {{ natureLabel(scheme.nature) }}
              </ElTag>
              <ElTag
                v-if="scheme.readonly"
                size="small"
                effect="plain"
                type="info"
              >
                只读
              </ElTag>
            </div>
          </div>
          <div class="item-actions" @click.stop>
            <ElSwitch
              :model-value="scheme.enabled"
              size="small"
              @change="(value) => toggleEnabled(scheme, Boolean(value))"
            />
            <ElDropdown
              trigger="click"
              @command="(cmd: string) => onMoreCommand(scheme, cmd)"
            >
              <button
                type="button"
                class="more-btn"
                title="更多"
                @click.stop
              >
                ···
              </button>
              <template #dropdown>
                <ElDropdownMenu>
                  <ElDropdownItem command="rename">重命名</ElDropdownItem>
                  <ElDropdownItem
                    command="keep"
                    :disabled="scheme.nature === 'keep'"
                  >
                    性质：保留
                  </ElDropdownItem>
                  <ElDropdownItem
                    command="exclusive"
                    :disabled="scheme.nature === 'exclusive'"
                  >
                    性质：单开
                  </ElDropdownItem>
                  <ElDropdownItem command="delete" divided>
                    删除
                  </ElDropdownItem>
                </ElDropdownMenu>
              </template>
            </ElDropdown>
          </div>
        </div>
        <p v-if="schemes.length === 0" class="empty">暂无方案，请拉取最新配置</p>
      </aside>

      <section class="editor">
        <template v-if="hasEditorTarget">
          <div class="editor-bar">
            <ElInput
              v-model="draftTitle"
              class="title-input"
              placeholder="方案标题"
              :disabled="viewingSystem"
              @input="markDirty"
            />
            <template v-if="metaEditable">
              <ElButton :disabled="!dirty" @click="discardEdits">
                放弃更改
              </ElButton>
              <ElButton
                type="primary"
                :loading="saving"
                :disabled="!dirty"
                @click="saveScheme"
              >
                保存
              </ElButton>
            </template>
            <ElTag v-else type="info" size="small" effect="plain">只读</ElTag>
          </div>
          <div v-if="metaEditable" class="remote-bar">
            <ElSelect
              v-model="draftType"
              class="type-select"
              @change="markDirty"
            >
              <ElOption label="本地" value="local" />
              <ElOption label="远程" value="remote" />
            </ElSelect>
            <template v-if="draftType === 'remote'">
              <ElInput
                v-model="draftUrl"
                class="url-input"
                placeholder="远程 URL"
                @input="markDirty"
              />
              <ElSelect
                v-model="draftInterval"
                class="interval-select"
                @change="markDirty"
              >
                <ElOption
                  v-for="opt in REFRESH_OPTIONS"
                  :key="opt.value"
                  :label="opt.label"
                  :value="opt.value"
                />
              </ElSelect>
              <ElButton
                :loading="refreshingRemote"
                :disabled="dirty || !draftUrl.trim()"
                @click="refreshRemoteNow"
              >
                立即刷新
              </ElButton>
            </template>
          </div>
          <p
            v-if="isRemoteSelected && selected && schemeLastRefresh(selected)"
            class="last-refresh"
          >
            上次刷新：{{ schemeLastRefresh(selected) }}
          </p>
          <div class="editor-body">
            <MonacoEditor
              v-model="draftContent"
              language="hosts"
              :read-only="contentReadOnly"
              placeholder="hosts 内容，例如：127.0.0.1 example.com"
              @update:model-value="markDirty"
            />
          </div>
        </template>
        <p v-else class="empty editor-empty">选择左侧方案进行编辑</p>
      </section>
    </div>

    <ElDialog
      v-model="createDialogVisible"
      title="新建 Host 方案"
      width="420px"
      append-to-body
    >
      <div class="create-nature">
        <div class="create-label">类型</div>
        <ElRadioGroup v-model="createType">
          <ElRadio value="local">本地</ElRadio>
          <ElRadio value="remote">远程</ElRadio>
        </ElRadioGroup>
        <template v-if="createType === 'remote'">
          <div class="create-label">远程 URL</div>
          <ElInput v-model="createUrl" placeholder="https://example.com/hosts" />
          <div class="create-label">定时刷新</div>
          <ElSelect v-model="createInterval" style="width: 100%">
            <ElOption
              v-for="opt in REFRESH_OPTIONS"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </ElSelect>
          <p class="create-hint">
            即使选择「永不」，首次创建时也会自动拉取一次远程内容。
          </p>
        </template>
        <div class="create-label">性质</div>
        <ElRadioGroup v-model="createNature">
          <ElRadio value="exclusive">单开</ElRadio>
          <ElRadio value="keep">保留</ElRadio>
        </ElRadioGroup>
        <p class="create-hint">
          单开：开启后自动关闭其他单开方案。保留：仅切换自身开关。
        </p>
      </div>
      <template #footer>
        <ElButton @click="createDialogVisible = false">取消</ElButton>
        <ElButton type="primary" :loading="saving" @click="confirmCreate">
          创建
        </ElButton>
      </template>
    </ElDialog>

    <ElDialog
      v-model="deleteDialogVisible"
      title="删除确认"
      width="400px"
      append-to-body
      align-center
    >
      <p class="delete-text">
        确定删除方案「{{ deleteTarget?.title }}」？
      </p>
      <template #footer>
        <ElButton @click="deleteDialogVisible = false">取消</ElButton>
        <ElButton
          type="danger"
          :loading="deleting"
          @click="confirmDeleteScheme"
        >
          删除
        </ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-input {
  display: none;
}

.body {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.list,
.editor {
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  background: var(--el-bg-color);
  min-height: 0;
}

.list {
  overflow: auto;
  padding: 8px;
}

.item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 8px;
  border-radius: 6px;
  cursor: pointer;
}

.item-system {
  position: sticky;
  top: 0;
  z-index: 2;
  margin-bottom: 6px;
  background: var(--el-bg-color);
  border: 1px solid var(--el-border-color);
  box-shadow: 0 1px 0 var(--el-bg-color);
}

.item:hover {
  background: var(--el-fill-color-light);
}

.item.active {
  background: var(--el-color-primary-light-9);
}

.item-main {
  min-width: 0;
  flex: 1;
}

.item-title {
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rename-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.rename-ok {
  flex-shrink: 0;
  padding: 5px 8px;
}

.item-meta {
  margin-top: 4px;
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  min-width: 0;
  overflow: hidden;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.item-meta :deep(.el-tag) {
  flex-shrink: 0;
  height: 18px;
  padding: 0 4px;
  font-size: 11px;
  line-height: 16px;
}

.meta-text {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.item-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  padding-top: 2px;
}

.more-btn {
  box-sizing: border-box;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--el-border-color);
  border-radius: 50%;
  background: transparent;
  color: var(--el-text-color-regular);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 1px;
  line-height: 1;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.more-btn:hover {
  border-color: var(--el-color-primary);
  color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}

.editor {
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 10px;
}

.editor-bar {
  display: flex;
  align-items: center;
  gap: 8px;
}

.remote-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.type-select {
  width: 100px;
  flex-shrink: 0;
}

.url-input {
  flex: 1;
  min-width: 160px;
}

.interval-select {
  width: 120px;
  flex-shrink: 0;
}

.last-refresh {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.title-input {
  flex: 1;
}

.editor-body {
  flex: 1;
  min-height: 0;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.empty {
  margin: 16px 8px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.editor-empty {
  margin: auto;
}

.create-nature {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.create-label {
  font-size: 14px;
  font-weight: 600;
}

.create-hint {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.5;
}

.delete-text {
  margin: 0;
  font-size: 14px;
  line-height: 1.6;
}

.perm-btn.perm-breathe {
  animation: perm-breathe 1.4s ease-in-out infinite;
  box-shadow: 0 0 0 0 rgba(245, 108, 108, 0.7);
}

@keyframes perm-breathe {
  0% {
    box-shadow: 0 0 0 0 rgba(245, 108, 108, 0.75);
  }
  55% {
    box-shadow: 0 0 0 8px rgba(245, 108, 108, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(245, 108, 108, 0);
  }
}
</style>
