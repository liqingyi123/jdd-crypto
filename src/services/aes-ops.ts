import { AES_ENUM } from "@/utils/aes-enum";
import { aesDecrypt, aesEncrypt } from "@/utils/crypto-aes";

export type AesOpType = "encrypt" | "decrypt" | "tokv";

export interface AesOpResult {
  code: "ok" | "error" | "empty";
  content: string;
  usedCode?: string;
  usedKey?: string;
  usedIv?: string;
}

export interface RunAesOptions {
  type: AesOpType;
  text: string;
  aesCode: string;
  customKey: string;
  customIv: string;
  /** When false, nested objects beyond depth 1 stay JSON strings in tokv. Default true. */
  deepObj?: boolean;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Object.prototype.toString.call(value) === "[object Object]";
}

function isArray(value: unknown): value is unknown[] {
  return Array.isArray(value);
}

/** Strip quotes / request= noise before decodeURIComponent (legacy formatEncodeStr). */
export function formatEncodeStr(str: string): string {
  return (
    str
      ?.replace(/"|'|‘|’|“|”|`|:|：|\s|\n/g, "")
      ?.replace("request=", "")
      ?.replace("request", "") || ""
  );
}

/**
 * Collapse prettified JSON before encrypt.
 * Lets users copy formatted decrypt output back into input and re-encrypt consistently.
 */
function compactForEncrypt(text: string): string {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
    return text;
  }
  try {
    return JSON.stringify(JSON.parse(trimmed));
  } catch {
    return text;
  }
}

function tryParse(txt: unknown, retried = false): unknown {
  if (typeof txt !== "string") {
    return txt;
  }
  try {
    return JSON.parse(txt);
  } catch {
    if (!retried) {
      return tryParse(txt.replace(/'/g, '"'), true);
    }
    return txt;
  }
}

function decryptRaw(text: string, key: string, iv: string): string {
  const cleaned = decodeURIComponent(formatEncodeStr(text));
  let res = aesDecrypt(cleaned, key, iv);
  res = res.replace(/^['|"](.*)['|"]$/, "$1");
  return res;
}

function decryptText(text: string, key: string, iv: string): AesOpResult {
  if (!text || !key || !iv) {
    return { code: "empty", content: "" };
  }
  try {
    const res = decryptRaw(text, key, iv);
    if (!res) {
      return {
        code: "error",
        content: "解密失败！请确认AES的key和iv是否正确！",
      };
    }
    // Keep raw decrypt bytes for encrypt round-trip; use editor "美化" for pretty JSON.
    try {
      JSON.parse(res);
      return { code: "ok", content: res };
    } catch {
      return {
        code: "ok",
        content: res.replace(/\\"/g, '"').replace(/\\n/g, "\n"),
      };
    }
  } catch {
    return {
      code: "error",
      content: "解密失败！请确认AES的key和iv是否正确！",
    };
  }
}

function toKv(
  text: string,
  key: string,
  iv: string,
  deepObj: boolean,
): AesOpResult {
  if (!text || !key || !iv) {
    return { code: "empty", content: "" };
  }
  try {
    let temp: unknown = decryptRaw(text, key, iv);
    if (typeof temp === "string" && !temp) {
      return {
        code: "error",
        content:
          "转换失败！1.请确认AES的key和iv是否正确！\n2.请确认解密后是否为json格式",
      };
    }
    temp = tryParse(temp);
    if (typeof temp === "string") {
      temp = temp.replace(/\\"/g, '"').replace(/\\n/g, "");
      temp = tryParse(temp);
    }
    if (!isPlainObject(temp)) {
      return {
        code: "error",
        content:
          "转换失败！1.请确认AES的key和iv是否正确！\n2.请确认解密后是否为json格式",
      };
    }

    const resArr: string[] = [];
    const pushkv = (k: string, value: unknown, deep: string) => {
      const lastI = resArr.findIndex((v) => v.split(":")[0] === k);
      const rendered = `${k}:${value === "" ? "" : value}`;
      if (lastI > -1) {
        if (deep.includes("header")) {
          resArr[lastI] = rendered;
        }
      } else {
        resArr.push(rendered);
      }
    };

    const openkv = (k: string, value: unknown, deep = "1") => {
      let next = value;
      if (typeof next === "string") {
        try {
          next = JSON.parse(next);
        } catch {
          // keep string
        }
      }
      if (isPlainObject(next)) {
        const depth = Number(deep.split("-")[0]);
        if (!deepObj && depth > 1) {
          pushkv(k, JSON.stringify(next), deep);
        } else {
          let depthCursor = depth;
          for (const childKey of Object.keys(next)) {
            openkv(childKey, next[childKey], `${depthCursor++}-${k}`);
          }
        }
      } else if (isArray(next)) {
        pushkv(k, JSON.stringify(next), deep);
      } else if (deep === "header" && k.toLocaleLowerCase() === "userid") {
        pushkv(k, next, deep);
        pushkv("tokenuserId", next, deep);
      } else {
        pushkv(k, next, deep);
      }
    };

    for (const k of Object.keys(temp)) {
      openkv(k, temp[k], "1");
    }
    return { code: "ok", content: resArr.join("\n") };
  } catch {
    return {
      code: "error",
      content:
        "转换失败！1.请确认AES的key和iv是否正确！\n2.请确认解密后是否为json格式",
    };
  }
}

/** Decode encrypt output for display: %2B → + (and other encodeURI sequences). */
function decodeCiphertextForDisplay(cipher: string): string {
  try {
    return decodeURIComponent(cipher);
  } catch {
    return cipher.replace(/%2B/gi, "+");
  }
}

function encryptText(text: string, key: string, iv: string): AesOpResult {
  if (!text || !key || !iv) {
    return { code: "empty", content: "" };
  }
  try {
    return {
      code: "ok",
      content: decodeCiphertextForDisplay(
        aesEncrypt(compactForEncrypt(text), key, iv),
      ),
    };
  } catch {
    return {
      code: "error",
      content: "加密失败！请确认AES的key和iv是否正确！",
    };
  }
}

function runWithKey(
  type: AesOpType,
  text: string,
  key: string,
  iv: string,
  deepObj: boolean,
): AesOpResult {
  if (type === "encrypt") {
    return encryptText(text, key, iv);
  }
  if (type === "tokv") {
    return toKv(text, key, iv, deepObj);
  }
  return decryptText(text, key, iv);
}

function resolvePreset(
  code: string,
  customKey: string,
  customIv: string,
): { code: string; key: string; iv: string } | null {
  if (code === "custom") {
    return { code: "custom", key: customKey, iv: customIv };
  }
  const found = AES_ENUM.find((item) => item.code === code);
  if (!found || found.code === "auto") {
    return null;
  }
  return { code: found.code, key: found.key, iv: found.iv };
}

/** Run encrypt / decrypt / tokv with optional auto-detect across presets. */
export function runAes(options: RunAesOptions): AesOpResult {
  const {
    type,
    text,
    aesCode,
    customKey,
    customIv,
    deepObj = true,
  } = options;

  if (aesCode === "auto") {
    let lastError: AesOpResult = {
      code: "error",
      content:
        type === "tokv"
          ? "转换失败！1.请确认AES的key和iv是否正确！\n2.请确认解密后是否为json格式"
          : type === "encrypt"
            ? "加密失败！请确认AES的key和iv是否正确！"
            : "解密失败！请确认AES的key和iv是否正确！",
    };

    for (const preset of AES_ENUM) {
      if (preset.code === "auto") {
        continue;
      }
      const key = preset.code === "custom" ? customKey : preset.key;
      const iv = preset.code === "custom" ? customIv : preset.iv;
      if (!key || !iv) {
        continue;
      }
      const result = runWithKey(type, text, key, iv, deepObj);
      if (result.code === "empty") {
        return result;
      }
      if (result.code !== "error") {
        return {
          ...result,
          usedCode: preset.code,
          usedKey: key,
          usedIv: iv,
        };
      }
      lastError = result;
    }
    return lastError;
  }

  const resolved = resolvePreset(aesCode, customKey, customIv);
  if (!resolved) {
    return { code: "error", content: "未找到对应的 AES 配置" };
  }
  const result = runWithKey(type, text, resolved.key, resolved.iv, deepObj);
  return {
    ...result,
    usedCode: resolved.code,
    usedKey: resolved.key,
    usedIv: resolved.iv,
  };
}
