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
    title: "剪贴板智能解密",
    summary:
      "监听到文本复制后静默尝试解密；仅当解密成功且结果为 JSON 时，在光标旁弹出结果气泡（不再因篇幅过长改开主界面），否则不打扰。",
  },
  {
    id: "bubble",
    title: "结果气泡",
    summary:
      "解密或加密结果在光标附近展示美化内容；支持拖动、一键复制（不重复触发剪贴板逻辑），失败可回退主界面处理。",
  },
  {
    id: "follow",
    title: "鼠标跟随选文",
    summary:
      "默认 Ctrl+Alt+G 开启后角标跟随光标；拖选或双击选中文本后自动复制，并在结果气泡中优先解密（失败再尝试加密）。快捷键可在设置中自定义。",
  },
  {
    id: "compare",
    title: "文本对比模式",
    summary:
      "默认 Ctrl+Alt+D 进入对比模式，跟随提示引导两次框选解密；成功后在中央左右对照展示，并高亮后段差异。快捷键可在设置中自定义。",
  },
  {
    id: "hosts",
    title: "Host 管理",
    summary:
      "内网固定预置方案，一键拉取最新配置；本地与服务器同域名冲突时以合并块提示自行取舍。支持保留 / 单开写入系统 hosts，可重置清理托管内容；可开关全局快捷键（默认 Ctrl+Alt+S）在光标旁快速切换方案，并弹出轻提示。",
  },
  {
    id: "brightness",
    title: "屏幕亮度调节",
    summary:
      "默认 Ctrl+Alt+L 在光标旁打开亮度气泡，为每个显示器显示进度条，拖动即可调节该屏物理亮度（笔记本 WMI / 外接屏 DDC/CI）。",
  },
  {
    id: "screensaver",
    title: "多屏屏保",
    summary:
      "默认 Ctrl+Alt+P 在所有显示器全屏显示屏保；背景可选海景视差、日冕生辉、雪国遗踪，时钟可选 3D 液晶 / 经典指针 / 文字罗盘。任意键、点击鼠标或再次快捷键退出；开启期间暂时关闭鼠标轨迹。",
  },
  {
    id: "trail",
    title: "鼠标拖尾特效",
    summary:
      "躁动线条、星痕漫衍、街头涂鸦、浮络牵光、绮心逐迹、沧涟曳逝（连贯水波纹）等多种特效，部分支持自定义颜色；可用 Ctrl+T+数字快速切换。",
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
