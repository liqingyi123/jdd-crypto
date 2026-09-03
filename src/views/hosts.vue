<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";
import MonacoEditor from "@/components/monaco-editor.vue";

type HostNature = "keep" | "exclusive";

const SYSTEM_ID = "__system__";

interface HostsScheme {
  id: string;
  title: string;
  content: string;
  enabled: boolean;
  source: string;
  nature: HostNature;
  readonly: boolean;
}

interface ImportResult {
  imported: number;
  skipped: number;
  schemes: HostsScheme[];
}

const loading = shallowRef(false);
const saving = shallowRef(false);
const importing = shallowRef(false);
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
const deleteDialogVisible = shallowRef(false);
const deleteTarget = shallowRef<HostsScheme | null>(null);
const deleting = shallowRef(false);
const openingSystem = shallowRef(false);
const hostsWritable = shallowRef(false);
const sessionPermissionOk = shallowRef(false);
const permissionBreathing = shallowRef(false);
const requestingPermission = shallowRef(false);
const renameInputRef = ref<{ focus?: () => void; input?: HTMLInputElement } | null>(
  null,
);

let stopRenameOutsideListen: (() => void) | undefined;

const selected = computed(
  () => schemes.value.find((s) => s.id === selectedId.value) ?? null,
);

const viewingSystem = computed(() => selectedId.value === SYSTEM_ID);

const editorReadOnly = computed(
  () => viewingSystem.value || selected.value?.readonly === true,
);

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
  if (syncing.value || editorReadOnly.value) {
    return;
  }
  dirty.value = true;
}

async function refresh() {
  loading.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_list");
    schemes.value = Array.isArray(list) ? list : [];
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
  createDialogVisible.value = true;
}

async function confirmCreate() {
  saving.value = true;
  try {
    const list = await invoke<HostsScheme[]>("hosts_upsert", {
      id: null,
      title: "新方案",
      content: "",
      enabled: false,
      nature: createNature.value,
    });
    schemes.value = list;
    createDialogVisible.value = false;
    const created = list[list.length - 1];
    if (created) {
      await selectScheme(created);
    }
    ElMessage.success("已新建方案");
  } catch (err) {
    ElMessage.error(String(err));
  } finally {
    saving.value = false;
  }
}

async function saveScheme() {
  if (!selectedId.value || editorReadOnly.value) {
    return;
  }
  const title = draftTitle.value.trim();
  if (!title) {
    ElMessage.warning("标题不能为空");
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
    });
    schemes.value = list;
    dirty.value = false;
    ElMessage.success("已保存");
    if (selected.value?.enabled) {
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

async function discardEdits() {
  if (!selected.value) {
    return;
  }
  syncing.value = true;
  draftTitle.value = selected.value.title;
  draftContent.value = selected.value.content;
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
    schemes.value = list;
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
    schemes.value = list;
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
    schemes.value = list;
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
    schemes.value = list;
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
    schemes.value = result.schemes;
    if (!selectedId.value && result.schemes.length > 0) {
      await selectScheme(result.schemes[result.schemes.length - 1]);
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

onMounted(() => {
  void refresh();
  void refreshHostsPermission();
});
</script>

<template>
  <div v-loading="loading" class="page">
    <div class="toolbar">
      <div class="toolbar-left">
        <ElButton type="primary" :loading="saving" @click="openCreateDialog">
          新建
        </ElButton>
        <ElButton :loading="importing" @click="triggerImport">
          导入
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
        <p v-if="schemes.length === 0" class="empty">暂无方案，可新建或导入</p>
      </aside>

      <section class="editor">
        <template v-if="hasEditorTarget">
          <div class="editor-bar">
            <ElInput
              v-model="draftTitle"
              class="title-input"
              placeholder="方案标题"
              :disabled="editorReadOnly"
              @input="markDirty"
            />
            <template v-if="!editorReadOnly">
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
          <div class="editor-body">
            <MonacoEditor
              v-model="draftContent"
              language="hosts"
              :read-only="editorReadOnly"
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
      width="360px"
      append-to-body
    >
      <div class="create-nature">
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
  margin-top: 6px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
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
