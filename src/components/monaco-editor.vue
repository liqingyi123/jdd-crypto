<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { storeToRefs } from "pinia";
// Slim entry: editor API only (avoids shipping every language bundle).
import * as monaco from "monaco-editor/editor/editor.api";
// Register JS grammar so result pane can highlight / fold object-like output.
import "monaco-editor/languages/definitions/javascript/register";
import { useThemeStore } from "@/stores/theme";
import { registerHostsLanguage } from "@/utils/monaco-hosts-language";

const model = defineModel<string>({ default: "" });

const emit = defineEmits<{
  clear: [];
}>();

const props = withDefaults(
  defineProps<{
    readOnly?: boolean;
    language?: string;
    placeholder?: string;
    folding?: boolean;
    clearOnDoubleClick?: boolean;
  }>(),
  {
    readOnly: false,
    language: "plaintext",
    folding: true,
    clearOnDoubleClick: false,
  },
);

const hostRef = ref<HTMLElement | null>(null);
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null);
const themeStore = useThemeStore();
const { resolved } = storeToRefs(themeStore);
let shellLanguageReady = false;

function monacoTheme(theme: string): string {
  return theme === "dark" ? "vs-dark" : "vs";
}

onMounted(async () => {
  if (!hostRef.value) {
    return;
  }
  if (props.language === "hosts") {
    registerHostsLanguage(monaco);
  } else if (props.language === "shell") {
    await import("monaco-editor/languages/definitions/shell/register");
    shellLanguageReady = true;
  }
  const instance = monaco.editor.create(hostRef.value, {
    value: model.value,
    language: props.language,
    readOnly: props.readOnly,
    automaticLayout: true,
    fontSize: 13,
    lineNumbers: "on",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    wordWrap: "on",
    theme: monacoTheme(resolved.value),
    stickyScroll: { enabled: false },
    folding: props.folding,
    showFoldingControls: props.folding ? "always" : "never",
    foldingStrategy: "auto",
  });
  editor.value = instance;

  instance.onDidChangeModelContent(() => {
    if (props.readOnly) {
      return;
    }
    const next = instance.getValue();
    if (next !== model.value) {
      model.value = next;
    }
  });

  instance.onMouseDown((event) => {
    if (
      !props.clearOnDoubleClick ||
      props.readOnly ||
      event.event.detail !== 2
    ) {
      return;
    }
    event.event.preventDefault();
    instance.setValue("");
    model.value = "";
    emit("clear");
  });
});

watch(model, (value) => {
  const instance = editor.value;
  if (!instance) {
    return;
  }
  if (instance.getValue() !== value) {
    instance.setValue(value ?? "");
  }
});

watch(
  () => props.language,
  async (language) => {
    if (language === "hosts") {
      registerHostsLanguage(monaco);
    } else if (language === "shell" && !shellLanguageReady) {
      await import("monaco-editor/languages/definitions/shell/register");
      shellLanguageReady = true;
    }
    const instance = editor.value;
    const modelRef = instance?.getModel();
    if (modelRef) {
      monaco.editor.setModelLanguage(modelRef, language);
    }
  },
);

watch(
  () => props.readOnly,
  (readOnly) => {
    editor.value?.updateOptions({ readOnly });
  },
);

watch(
  () => props.folding,
  (folding) => {
    editor.value?.updateOptions({
      folding,
      showFoldingControls: folding ? "always" : "never",
    });
  },
);

watch(resolved, (theme) => {
  monaco.editor.setTheme(monacoTheme(theme));
});

function revealMatchingLine(text: string): boolean {
  const instance = editor.value;
  const modelRef = instance?.getModel();
  if (!instance || !modelRef) {
    return false;
  }
  const needle = text.trim();
  if (!needle) {
    return false;
  }
  const lineCount = modelRef.getLineCount();
  let matchLine = 0;
  for (let i = 1; i <= lineCount; i += 1) {
    if (modelRef.getLineContent(i).trim() === needle) {
      matchLine = i;
      break;
    }
  }
  if (matchLine === 0) {
    for (let i = 1; i <= lineCount; i += 1) {
      if (modelRef.getLineContent(i).includes(needle)) {
        matchLine = i;
        break;
      }
    }
  }
  if (matchLine === 0) {
    return false;
  }
  const maxCol = modelRef.getLineMaxColumn(matchLine);
  instance.revealLineInCenter(matchLine);
  instance.setSelection({
    startLineNumber: matchLine,
    startColumn: 1,
    endLineNumber: matchLine,
    endColumn: maxCol,
  });
  instance.focus();
  return true;
}

defineExpose({
  revealMatchingLine,
});

onBeforeUnmount(() => {
  editor.value?.dispose();
  editor.value = null;
});
</script>

<template>
  <div class="monaco-host" :data-placeholder="placeholder">
    <div ref="hostRef" class="monaco-editor-root" />
  </div>
</template>

<style scoped>
.monaco-host {
  flex: 1;
  min-height: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-elevated);
}

.monaco-editor-root {
  width: 100%;
  height: 100%;
  min-height: 0;
}
</style>
