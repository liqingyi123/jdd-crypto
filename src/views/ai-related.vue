<script setup lang="ts">
import { computed, onMounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ElMessage, ElMessageBox } from "element-plus";
import MonacoEditor from "@/components/monaco-editor.vue";

type TeamaiAction =
  | "deploy"
  | "upgrade"
  | "pull"
  | "push"
  | "disableAutoUpgrade"
  | "enableAutoUpgrade";

type UpdatePolicy = "auto" | "prompt" | "skip";

interface TeamaiPref {
  repoUrl: string;
  accessToken: string;
}

interface TeamaiCliStatus {
  installed: boolean;
  bin: string | null;
  log: string;
}

interface TeamaiRunResult {
  ok: boolean;
  log: string;
}

interface TeamaiUpgradeResult {
  ok: boolean;
  upToDate: boolean;
  version: string | null;
  log: string;
}

interface TeamaiDeployResult {
  ok: boolean;
  version: string | null;
  log: string;
}

const DEFAULT_REPO_URL =
  "http://git.jiangduoduo.com/qianduan/ai-standards.git";
const TOKEN_PAGE_URL =
  "https://git.jiangduoduo.com/-/profile/personal_access_tokens";

const ACTION_LABELS: Record<TeamaiAction, string> = {
  deploy: "一键部署",
  upgrade: "升级版本",
  pull: "拉取配置",
  push: "上传配置",
  disableAutoUpgrade: "禁用自动升级",
  enableAutoUpgrade: "启用自动升级",
};

const repoUrl = shallowRef(DEFAULT_REPO_URL);
const accessToken = shallowRef("");
const cliInstalled = shallowRef(false);
const cliVersion = shallowRef("");
const updatePolicy = shallowRef<UpdatePolicy | null>(null);
const openingLocalDir = shallowRef(false);
const pushDialogVisible = shallowRef(false);
const pushBranch = shallowRef("");
const pushMessage = shallowRef("");
const logHint = shallowRef("");
const actionLoading = shallowRef<TeamaiAction | null>(null);
const activeAction = shallowRef<TeamaiAction>("deploy");
const logs = shallowRef<Record<TeamaiAction, string>>({
  deploy: "",
  upgrade: "",
  pull: "",
  push: "",
  disableAutoUpgrade: "",
  enableAutoUpgrade: "",
});

const logText = computed({
  get: () => logs.value[activeAction.value] ?? "",
  set: (value: string) => {
    logs.value = { ...logs.value, [activeAction.value]: value };
  },
});

const versionLabel = computed(() => {
  const v = cliVersion.value.trim();
  if (!v) {
    return "";
  }
  return v.startsWith("v") ? v : `v${v}`;
});

const autoUpgradeOff = computed(() => updatePolicy.value === "skip");

const visibleActions = computed(() => {
  if (!cliInstalled.value) {
    return [{ id: "deploy" as const, label: "一键部署" }];
  }
  const actions: Array<{
    id: TeamaiAction;
    label: string;
    disabled?: boolean;
  }> = [
    { id: "deploy", label: "重新部署" },
    { id: "upgrade", label: ACTION_LABELS.upgrade },
    { id: "pull", label: ACTION_LABELS.pull },
    { id: "push", label: ACTION_LABELS.push },
  ];
  if (autoUpgradeOff.value) {
    actions.push({
      id: "enableAutoUpgrade",
      label: ACTION_LABELS.enableAutoUpgrade,
    });
  } else {
    actions.push({
      id: "disableAutoUpgrade",
      label: ACTION_LABELS.disableAutoUpgrade,
    });
  }
  return actions;
});

function setActionLog(action: TeamaiAction, text: string) {
  activeAction.value = action;
  logs.value = { ...logs.value, [action]: text };
}

async function loadPref() {
  try {
    const pref = await invoke<TeamaiPref>("get_teamai_pref");
    repoUrl.value = pref.repoUrl || DEFAULT_REPO_URL;
    accessToken.value = pref.accessToken ?? "";
  } catch {
    repoUrl.value = DEFAULT_REPO_URL;
    accessToken.value = "";
  }
}

async function openTokenPage() {
  try {
    await openUrl(TOKEN_PAGE_URL);
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
}

async function openLocalDir() {
  if (openingLocalDir.value) {
    return;
  }
  openingLocalDir.value = true;
  try {
    await invoke("open_teamai_local_dir");
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
  } finally {
    openingLocalDir.value = false;
  }
}

async function refreshVersion(): Promise<string | null> {
  try {
    const ver = await invoke<string>("teamai_cli_version");
    cliVersion.value = ver;
    return ver;
  } catch (error) {
    cliVersion.value = "";
    return error instanceof Error ? error.message : String(error);
  }
}

async function refreshUpdatePolicy() {
  try {
    const policy = await invoke<UpdatePolicy | null>("teamai_cli_get_update_policy");
    updatePolicy.value =
      policy === "auto" || policy === "prompt" || policy === "skip" ? policy : null;
  } catch {
    updatePolicy.value = null;
  }
}

async function detectCli() {
  let status: TeamaiCliStatus = {
    installed: false,
    bin: null,
    log: "# teamai CLI 检测失败（invoke 异常）\n",
  };
  try {
    status = await invoke<TeamaiCliStatus>("is_teamai_cli_installed");
  } catch (error) {
    status.log += String(error instanceof Error ? error.message : error) + "\n";
  }
  cliInstalled.value = status.installed;
  const target: TeamaiAction = status.installed ? "upgrade" : "deploy";
  const stamp = new Date().toLocaleString();
  let body = `${status.log || ""}`;
  if (status.installed) {
    const verOrErr = await refreshVersion();
    if (!cliVersion.value && verOrErr) {
      body += `\n# 读取版本失败\n${verOrErr}\n`;
    }
    await refreshUpdatePolicy();
  } else {
    cliVersion.value = "";
    updatePolicy.value = null;
  }
  const header = `# [${stamp}] CLI 检测 → ${status.installed ? "已安装" : "未安装"}${
    status.bin ? ` (${status.bin})` : ""
  }${cliVersion.value ? ` version=${cliVersion.value}` : ""}\n`;
  setActionLog(target, `${header}${body}`);
}

async function onDeploy() {
  const repo = repoUrl.value.trim();
  if (!repo) {
    ElMessage.warning("请填写仓库地址");
    return;
  }

  const token = accessToken.value.trim();
  if (!token) {
    try {
      await ElMessageBox.confirm(
        "访问令牌未填写，团队协作能力将不可用，是否继续？",
        "提示",
        {
          type: "warning",
          confirmButtonText: "继续部署",
          cancelButtonText: "继续填写",
        },
      );
    } catch {
      return;
    }
  }

  logHint.value = "";
  actionLoading.value = "deploy";
  try {
    const result = await invoke<TeamaiDeployResult>("teamai_cli_deploy", {
      repoUrl: repo,
      accessToken: token,
    });
    setActionLog("deploy", result.log || "");
    if (result.ok) {
      cliInstalled.value = true;
      if (result.version) {
        cliVersion.value = result.version;
      } else {
        await refreshVersion();
      }
      const ver = versionLabel.value || (cliVersion.value ? `v${cliVersion.value}` : "");
      logHint.value = ver
        ? `已成功部署Teamai(${ver})`
        : "已成功部署Teamai";
      ElMessage.success("部署完成");
    } else {
      if (result.version) {
        cliVersion.value = result.version;
      }
      ElMessage.error("部署失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setActionLog("deploy", `# 部署失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    actionLoading.value = null;
  }
}

async function onUpgrade() {
  logHint.value = "";
  actionLoading.value = "upgrade";
  try {
    const result = await invoke<TeamaiUpgradeResult>("teamai_cli_upgrade");
    setActionLog("upgrade", result.log || "");
    if (result.version) {
      cliVersion.value = result.version;
    } else if (result.ok) {
      await refreshVersion();
    }
    if (result.upToDate) {
      logHint.value = "暂无可升级版本";
    } else if (result.ok) {
      ElMessage.success("升级完成");
    } else {
      ElMessage.error("升级失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setActionLog("upgrade", `# 升级失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    actionLoading.value = null;
  }
}

async function onPull() {
  logHint.value = "";
  actionLoading.value = "pull";
  try {
    const result = await invoke<TeamaiRunResult>("teamai_cli_pull");
    setActionLog("pull", result.log || "");
    if (result.ok) {
      logHint.value = "配置更新完成";
      ElMessage.success("拉取完成");
    } else {
      ElMessage.error("拉取失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setActionLog("pull", `# 拉取失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    actionLoading.value = null;
  }
}

async function onPush() {
  actionLoading.value = "push";
  try {
    const hasChanges = await invoke<boolean>("teamai_cli_has_push_changes");
    if (!hasChanges) {
      ElMessage.warning("当前没有 rules/skills/mcp 的可推送变更");
      return;
    }
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : String(error));
    return;
  } finally {
    actionLoading.value = null;
  }

  pushBranch.value = "";
  pushMessage.value = "";
  pushDialogVisible.value = true;
}

async function confirmPush() {
  const branch = pushBranch.value.trim();
  const message = pushMessage.value.trim();
  if (!branch) {
    ElMessage.warning("请填写分支名");
    return;
  }
  if (/\s/.test(branch)) {
    ElMessage.warning("分支名不能包含空白字符");
    return;
  }
  if (!message) {
    ElMessage.warning("请填写推送描述");
    return;
  }

  pushDialogVisible.value = false;
  logHint.value = "";
  actionLoading.value = "push";
  try {
    const result = await invoke<TeamaiRunResult>("teamai_cli_push", {
      branch,
      message,
    });
    setActionLog("push", result.log || "");
    if (result.ok) {
      ElMessage.success("已推送分支，请到 GitLab 创建并合并 MR");
    } else {
      ElMessage.error("上传失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setActionLog("push", `# 上传失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    actionLoading.value = null;
  }
}

async function onSetAutoUpgrade(enable: boolean) {
  const action: TeamaiAction = enable ? "enableAutoUpgrade" : "disableAutoUpgrade";
  const policy = enable ? "auto" : "skip";
  logHint.value = "";
  actionLoading.value = action;
  try {
    const result = await invoke<TeamaiRunResult>("teamai_cli_set_update_policy", {
      policy,
    });
    setActionLog(action, result.log || "");
    if (result.ok) {
      updatePolicy.value = policy;
      logHint.value = enable ? "已启用自动升级" : "已禁用自动升级";
      ElMessage.success(logHint.value);
    } else {
      ElMessage.error("设置失败，详见执行日志");
    }
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    setActionLog(action, `# 设置自动升级失败\n${msg}\n`);
    ElMessage.error(msg);
  } finally {
    actionLoading.value = null;
  }
}

function onActionClick(action: TeamaiAction) {
  if (action === "deploy") {
    void onDeploy();
    return;
  }
  if (action === "upgrade") {
    void onUpgrade();
    return;
  }
  if (action === "pull") {
    void onPull();
    return;
  }
  if (action === "push") {
    void onPush();
    return;
  }
  if (action === "disableAutoUpgrade") {
    void onSetAutoUpgrade(false);
    return;
  }
  if (action === "enableAutoUpgrade") {
    void onSetAutoUpgrade(true);
  }
}

onMounted(async () => {
  await Promise.all([loadPref(), detectCli()]);
});
</script>

<template>
  <div class="page">
    <section class="teamai-section">
      <div class="title-row">
        <h2>Teamai</h2>
        <span v-if="versionLabel" class="version-badge">{{ versionLabel }}</span>
        <ElLink
          v-if="cliInstalled"
          type="primary"
          :underline="false"
          :disabled="openingLocalDir"
          class="open-local-link"
          @click="openLocalDir"
        >
          {{ openingLocalDir ? "打开中…" : "打开本地目录" }}
        </ElLink>
      </div>

      <div class="field-row">
        <label class="field-label" for="teamai-repo">仓库地址</label>
        <ElInput
          id="teamai-repo"
          v-model="repoUrl"
          clearable
          :placeholder="DEFAULT_REPO_URL"
        />
      </div>
      <div class="field-row">
        <label class="field-label" for="teamai-token">访问令牌</label>
        <ElInput
          id="teamai-token"
          v-model="accessToken"
          clearable
          placeholder="访问令牌"
        />
        <ElLink type="primary" :underline="false" @click="openTokenPage">
          获取令牌
        </ElLink>
      </div>

      <div class="action-row">
        <ElButton
          v-for="action in visibleActions"
          :key="action.id"
          :loading="actionLoading === action.id"
          :disabled="
            Boolean(action.disabled) ||
            (actionLoading !== null && actionLoading !== action.id)
          "
          @click="onActionClick(action.id)"
        >
          {{ action.label }}
        </ElButton>
      </div>

      <div class="log-head">
        <h3>执行日志</h3>
        <span v-if="logHint" class="log-hint">{{ logHint }}</span>
      </div>
      <div class="log-pane">
        <MonacoEditor
          v-model="logText"
          language="shell"
          read-only
          :folding="false"
          placeholder="点击上方按钮查看对应操作的执行日志"
        />
      </div>
    </section>

    <ElDialog
      v-model="pushDialogVisible"
      title="上传配置"
      width="480px"
      append-to-body
      :close-on-click-modal="false"
    >
      <div class="push-dialog-body">
        <div class="push-field">
          <label class="push-label" for="push-branch">分支名</label>
          <ElInput
            id="push-branch"
            v-model="pushBranch"
            clearable
            placeholder="例如：feature_20261013_addZhcxRules"
          />
        </div>
        <div class="push-field">
          <label class="push-label" for="push-message">推送描述</label>
          <ElInput
            id="push-message"
            v-model="pushMessage"
            type="textarea"
            :rows="4"
            placeholder="例如：新增中华彩讯项目 rules"
          />
        </div>
      </div>
      <template #footer>
        <ElButton @click="pushDialogVisible = false">取消</ElButton>
        <ElButton type="primary" @click="confirmPush">确定</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.teamai-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-elevated);
}

.title-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 16px;
}

h2 {
  margin: 0;
  font-size: 16px;
}

.version-badge {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-muted);
}

.open-local-link {
  font-size: 13px;
  margin-left: 4px;
}

.push-dialog-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.push-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.push-label {
  font-size: 13px;
  color: var(--text-muted);
}

.field-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.field-label {
  flex: 0 0 72px;
  font-size: 13px;
  color: var(--text-muted);
}

.field-row :deep(.el-input) {
  flex: 1;
  min-width: 0;
}

.field-row :deep(.el-link) {
  flex-shrink: 0;
  white-space: nowrap;
}

.action-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}

.log-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.log-head h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.log-hint {
  font-size: 12px;
  color: var(--text-muted);
}

.log-pane {
  flex: 1;
  min-height: 180px;
  display: flex;
  flex-direction: column;
}
</style>
