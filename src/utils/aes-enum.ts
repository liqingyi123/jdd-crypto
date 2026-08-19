export interface AesPreset {
  key: string;
  iv: string;
  who: string;
  code: string;
}

/** Platform AES presets. Keep in sync with the legacy jdd-crypto enum. */
export const AES_ENUM: AesPreset[] = [
  { key: "", iv: "", who: "自动识别", code: "auto" },
  {
    key: "d3YmI1BUOSE2S2YmalBVZUQ=",
    iv: "0000000000000000",
    who: "赛酷/彩虹APP",
    code: "skch",
  },
  {
    key: "DdZQOX3jhDKhKmXEpoTQaYai",
    iv: "0000000000000000",
    who: "海外体育BOSS平台",
    code: "hwtyboss",
  },
  {
    key: "DYxDuAAYgu7stBa4edsGcLYM",
    iv: "0000000000000000",
    who: "BOSS平台",
    code: "boss",
  },
  {
    key: "UXQmI1B5OSE2TmYmalBVZVg=",
    iv: "0000000000000000",
    who: "一起有数小程序",
    code: "yqys",
  },
  {
    key: "QipVAlnxoMjeQ4I47ne7kWDk",
    iv: "0000000000000000",
    who: "海外体育API",
    code: "hwty",
  },
  { key: "", iv: "", who: "自定义", code: "custom" },
];

export const DEFAULT_AES_CODE = "auto";
