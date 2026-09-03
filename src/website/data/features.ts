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
    summary:
      "监听到文本复制后，在鼠标旁弹出询问：加密、解密或忽略。短文本解密可直接出结果气泡，长文本再打开主界面。",
  },
  {
    id: "bubble",
    title: "短文本结果气泡",
    summary:
      "短密文一键解密后，在光标附近展示美化结果；支持拖动、一键复制（不重复弹出询问），失败可回退主界面处理。",
  },
  {
    id: "follow",
    title: "鼠标跟随选文",
    summary:
      "自定义快捷键开启后角标跟随光标；拖选或双击选中文本后自动复制，短文本走结果气泡，长文本代入主界面加解密。",
  },
  {
    id: "compare",
    title: "文本对比模式",
    summary:
      "快捷键进入对比模式，跟随提示引导两次框选解密；成功后在中央左右对照展示，并高亮后段差异。快捷键可在设置中自定义。",
  },
  {
    id: "trail",
    title: "鼠标拖尾特效",
    summary:
      "躁动线条、星痕漫衍、街头涂鸦、浮络牵光、绮心逐迹、沧涟曳逝等多种特效，部分支持自定义颜色；可用 Ctrl+T+数字快速切换。",
  },
  {
    id: "theme",
    title: "深浅色主题",
    summary: "跟随系统外观，也可强制浅色或深色，与编辑器、角标界面保持一致。",
  },
  {
    id: "autostart",
    title: "开机自启动",
    summary: "可在设置中开启登录后自动启动，角标与托盘随时待命。",
  },
  {
    id: "update",
    title: "内网检查更新",
    summary: "启动静默检查；关于页可手动检查，下载并安装 Windows / macOS 安装包。",
  },
];

export const TRAIL_EFFECT_OPTIONS = [
  { id: "ribbon" as const, label: "躁动线条" },
  { id: "meteor" as const, label: "星痕漫衍" },
  { id: "graffiti" as const, label: "街头涂鸦" },
  { id: "dots" as const, label: "浮络牵光" },
  { id: "heart" as const, label: "绮心逐迹" },
  { id: "ripple" as const, label: "沧涟曳逝" },
];
