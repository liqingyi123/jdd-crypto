<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { storeToRefs } from "pinia";
// Slim entry: editor API only (avoids shipping every language bundle).
import * as monaco from "monaco-editor/editor/editor.api";
// Register JS grammar so result pane can highlight / fold object-like output.
import "monaco-editor/languages/definitions/javascript/register";
import { useThemeStore } from "@/stores/theme";

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

function monacoTheme(theme: string): string {
  return theme === "dark" ? "vs-dark" : "vs";
}

onMounted(() => {
  if (!hostRef.value) {
    return;
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
  (language) => {
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
