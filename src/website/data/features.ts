export interface FeatureItem {
  id: string;
  title: string;
  summary: string;
}

export const FEATURE_ITEMS: FeatureItem[] = [
  {
    id: "badge",
    title: "悬浮角标",
    summary: "桌面常驻、可拖动的透明角标；左键打开加解密，右键呼出菜单，不打扰你的工作流。",
  },
  {
    id: "clipboard",
    title: "剪贴板智能询问",
    summary: "监听到文本复制后，在鼠标旁弹出询问：立即加密、解密或忽略，减少来回切换。",
  },
  {
    id: "follow",
    title: "鼠标跟随选文",
    summary: "快捷键开启后角标跟随光标；拖选或双击选中文本后自动复制并代入加解密页面。",
  },
  {
    id: "trail",
    title: "鼠标拖尾特效",
    summary: "躁动线条、绚丽流星、街头涂鸦、连线点阵、心动回忆等多种特效，部分支持自定义颜色。",
  },
  {
    id: "theme",
    title: "深浅色主题",
    summary: "跟随系统外观，也可强制浅色或深色，与编辑器、角标界面保持一致。",
  },
  // {
  //   id: "plugins",
  //   title: "插件扩展",
  //   summary: "后续将扩展编辑器主题与加解密预设。",
  // },
  {
    id: "update",
    title: "内网检查更新",
    summary: "启动静默检查；关于页可手动检查，下载并安装 Windows / macOS 安装包。",
  },
];

export const TRAIL_EFFECT_OPTIONS = [
  { id: "ribbon" as const, label: "躁动线条" },
  { id: "meteor" as const, label: "绚丽流星" },
  { id: "graffiti" as const, label: "街头涂鸦" },
  { id: "dots" as const, label: "连线点阵" },
  { id: "heart" as const, label: "心动回忆" },
];
