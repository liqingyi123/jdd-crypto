export function useWindowDrag() {
  return {
    dragAttrs: {
      "data-tauri-drag-region": "",
    } as const,
  };
}
