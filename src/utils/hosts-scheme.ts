export type HostNature = "keep" | "exclusive";
export type HostSchemeType = "local" | "remote";

export interface HostsScheme {
  id: string;
  title: string;
  content: string;
  enabled: boolean;
  source: string;
  nature: HostNature;
  readonly: boolean;
  /** App IPC: schemeType; SwitchHosts import source field: type */
  type?: HostSchemeType;
  schemeType?: HostSchemeType;
  url: string;
  refresh_interval?: number;
  refreshInterval?: number;
  last_refresh?: string;
  lastRefresh?: string;
  last_refresh_ms?: number;
  lastRefreshMs?: number;
}

export function normalizeScheme(raw: HostsScheme): HostsScheme {
  const url = (raw.url ?? "").trim();
  const rawType = String(raw.schemeType ?? raw.type ?? "").toLowerCase();
  const type: HostSchemeType =
    rawType === "remote" || url.length > 0 ? "remote" : "local";
  const refresh_interval = Number(
    raw.refreshInterval ?? raw.refresh_interval ?? 0,
  );
  const last_refresh = String(raw.lastRefresh ?? raw.last_refresh ?? "");
  const last_refresh_ms = Number(
    raw.lastRefreshMs ?? raw.last_refresh_ms ?? 0,
  );
  return {
    ...raw,
    type,
    schemeType: type,
    url,
    refresh_interval: Number.isFinite(refresh_interval) ? refresh_interval : 0,
    refreshInterval: Number.isFinite(refresh_interval) ? refresh_interval : 0,
    last_refresh,
    lastRefresh: last_refresh,
    last_refresh_ms: Number.isFinite(last_refresh_ms) ? last_refresh_ms : 0,
    lastRefreshMs: Number.isFinite(last_refresh_ms) ? last_refresh_ms : 0,
    readonly: type === "remote" ? true : !!raw.readonly,
  };
}

export function normalizeSchemes(list: HostsScheme[] | null | undefined): HostsScheme[] {
  if (!Array.isArray(list)) {
    return [];
  }
  return list.map((item) => normalizeScheme(item));
}
