import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  defaultPluginSlots,
  findSlot,
  PLUGIN_KIND_META,
  type PluginKind,
  type PluginSlotsState,
} from "@/constants/plugin-slots";

export function usePluginSlots() {
  const state = ref<PluginSlotsState>(defaultPluginSlots());
  const loading = ref(false);
  const importingKind = ref<PluginKind | null>(null);
  const resettingKind = ref<PluginKind | null>(null);

  async function refresh() {
    loading.value = true;
    try {
      state.value = await invoke<PluginSlotsState>("get_plugin_slots");
    } catch {
      state.value = defaultPluginSlots();
    } finally {
      loading.value = false;
    }
  }

  function slotOf(kind: PluginKind) {
    return findSlot(state.value, kind)!;
  }

  async function toggleSlot(kind: PluginKind, enabled: boolean) {
    const meta = PLUGIN_KIND_META[kind];
    if (meta.comingSoon) {
      ElMessage.warning("开发中，敬请期待");
      return;
    }
    try {
      state.value = await invoke<PluginSlotsState>("set_plugin_slot_enabled", {
        kind,
        enabled,
      });
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : String(error));
      await refresh();
    }
  }

  async function resetSlot(kind: PluginKind) {
    resettingKind.value = kind;
    try {
      state.value = await invoke<PluginSlotsState>("reset_plugin_slot", { kind });
      ElMessage.success("已恢复默认");
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : String(error));
    } finally {
      resettingKind.value = null;
    }
  }

  async function importSlot(kind: PluginKind, file: File) {
    const meta = PLUGIN_KIND_META[kind];
    if (meta.comingSoon) {
      ElMessage.warning("开发中，敬请期待");
      return;
    }

    const current = slotOf(kind);
    if (current.source === "imported" && current.current?.fileName) {
      try {
        await ElMessageBox.confirm(
          `将替换当前插件「${current.current.name}」，是否继续？`,
          "导入插件",
          { type: "warning", confirmButtonText: "继续", cancelButtonText: "取消" },
        );
      } catch {
        return;
      }
    }

    importingKind.value = kind;
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      state.value = await invoke<PluginSlotsState>("import_plugin", {
        kind,
        fileName: file.name,
        bytes: Array.from(bytes),
      });
      ElMessage.success("插件导入成功");
    } catch (error) {
      ElMessage.error(error instanceof Error ? error.message : String(error));
    } finally {
      importingKind.value = null;
    }
  }

  return {
    state,
    loading,
    importingKind,
    resettingKind,
    refresh,
    slotOf,
    toggleSlot,
    resetSlot,
    importSlot,
  };
}
