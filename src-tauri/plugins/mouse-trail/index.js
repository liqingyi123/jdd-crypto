/**
 * Mouse trail overlay plugin (stub).
 * Host will later call register() after permission checks.
 */
export function register(api) {
  api.registerOverlayEffect?.({
    id: "mouse-trail",
    label: "鼠标拖尾",
  });
}
