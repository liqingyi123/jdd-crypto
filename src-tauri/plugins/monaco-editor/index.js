/**
 * Monaco editor plugin (stub).
 * Later: dynamically import monaco and replace the default textarea.
 */
export function register(api) {
  api.registerEditor?.({
    id: "monaco-editor",
    label: "Monaco Editor",
  });
}
