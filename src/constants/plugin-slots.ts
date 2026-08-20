export type PluginKind = "editor-theme" | "crypto-preset";

export type PluginSource = "preset" | "imported";

export interface PluginSlotCurrent {
  id: string;
  name: string;
  fileName?: string;
}

export interface PluginSlot {
  kind: PluginKind;
  enabled: boolean;
  source: PluginSource;
  current: PluginSlotCurrent | null;
}

export interface PluginSlotsState {
  slots: PluginSlot[];
}

export const PLUGIN_KIND_META: Record<
  PluginKind,
  { title: string; description: string; comingSoon: boolean }
> = {
  "editor-theme": {
    title: "编辑器主题",
    description: "为 Monaco 编辑器导入自定义主题配色。",
    comingSoon: true,
  },
  "crypto-preset": {
    title: "加解密预设",
    description: "导入额外的密钥、IV 与算法预设组合。",
    comingSoon: true,
  },
};

export function defaultPluginSlots(): PluginSlotsState {
  return {
    slots: [
      {
        kind: "editor-theme",
        enabled: false,
        source: "preset",
        current: null,
      },
      {
        kind: "crypto-preset",
        enabled: false,
        source: "preset",
        current: null,
      },
    ],
  };
}

export function findSlot(
  state: PluginSlotsState,
  kind: PluginKind,
): PluginSlot | undefined {
  return state.slots.find((item) => item.kind === kind);
}
