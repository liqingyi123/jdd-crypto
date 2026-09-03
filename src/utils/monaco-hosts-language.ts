import type * as Monaco from "monaco-editor/editor/editor.api";

const LANGUAGE_ID = "hosts";
let registered = false;

/** Register a lightweight hosts highlighter (idempotent). */
export function registerHostsLanguage(monaco: typeof Monaco): void {
  if (registered) {
    return;
  }
  registered = true;

  monaco.languages.register({ id: LANGUAGE_ID });
  monaco.languages.setMonarchTokensProvider(LANGUAGE_ID, {
    defaultToken: "",
    tokenizer: {
      root: [
        [/#.*$/, "comment"],
        [
          /\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b/,
          "number",
        ],
        [/(?:[0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}/, "type"],
        [/[A-Za-z0-9][A-Za-z0-9._-]*/, "string"],
        [/\s+/, "white"],
      ],
    },
  });
}

export const HOSTS_LANGUAGE_ID = LANGUAGE_ID;
