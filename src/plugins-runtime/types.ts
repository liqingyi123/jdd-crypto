export type PluginPermission =
  | "clipboard.read"
  | "fs.read"
  | "window.overlay"
  | "ui.editor"
  | "crypto.options"
  | (string & {});

export interface CryptoOptionContribution {
  id: string;
  label: string;
  algorithm: string;
  keyId?: string;
  ivId?: string;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  entry: string;
  permissions: PluginPermission[];
  contributes: {
    cryptoOptions?: CryptoOptionContribution[];
    editor?: string;
    overlayEffect?: boolean;
  };
  enabled: boolean;
  dir: string;
}

export interface PluginHostApi {
  registerCryptoOption?: (option: CryptoOptionContribution) => void;
  registerEditor?: (editor: { id: string; label: string }) => void;
  registerOverlayEffect?: (effect: { id: string; label: string }) => void;
  requestOverlay?: () => Promise<boolean>;
}
