/**
 * Extra crypto key/iv presets plugin (stub).
 */
export function register(api) {
  api.registerCryptoOption?.({
    id: "aes-256-gcm-demo",
    label: "AES-256-GCM Demo",
    algorithm: "AES-256-GCM",
  });
}
