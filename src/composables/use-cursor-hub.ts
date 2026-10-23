import { onMounted, onUnmounted, shallowRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createFrameGate } from "@/effects/trail-frame-budget";

export interface CursorHubSnapshot {
  x: number;
  y: number;
  seq: number;
  monitor: number;
}

export interface CursorPayload {
  x: number;
  y: number;
}

function parseOverlayIndex(label: string, prefix: string): number | null {
  if (!label.startsWith(prefix)) {
    return null;
  }
  const raw = Number.parseInt(label.slice(prefix.length), 10);
  return Number.isFinite(raw) ? raw : null;
}

export function useCursorHub(options: {
  labelPrefix: "mouse-trail-" | "spotlight-";
  onCursor: (payload: CursorPayload) => void;
  onLeaveMonitor?: () => void;
  /** 0 = uncapped. Omit for spotlight (full refresh). */
  getTargetFps?: () => number;
}) {
  const activeMonitor = shallowRef<number | null>(null);
  let myIndex: number | null = null;
  let rafId = 0;
  let lastSeq = -1;
  let peeking = false;
  let unlistenMonitor: UnlistenFn | undefined;
  let stopped = false;
  const frameGate = createFrameGate();
  let appliedFps = 0;

  function stopRaf() {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
  }

  function isActiveMonitor(): boolean {
    if (myIndex === null) {
      return false;
    }
    if (activeMonitor.value === null) {
      return true;
    }
    return activeMonitor.value === myIndex;
  }

  function syncGateFps() {
    const next = options.getTargetFps?.() ?? 0;
    if (next === appliedFps) {
      return;
    }
    appliedFps = next;
    frameGate.setTargetFps(next);
  }

  async function tick(now: number) {
    if (stopped) {
      return;
    }
    if (!isActiveMonitor()) {
      stopRaf();
      options.onLeaveMonitor?.();
      return;
    }
    syncGateFps();
    if (frameGate.shouldRun(now) && !peeking) {
      peeking = true;
      try {
        const snap = await invoke<CursorHubSnapshot>("peek_cursor_hub");
        if (stopped) {
          return;
        }
        if (activeMonitor.value === null) {
          activeMonitor.value = snap.monitor;
        }
        if (myIndex !== null && snap.monitor !== myIndex) {
          activeMonitor.value = snap.monitor;
          options.onLeaveMonitor?.();
          peeking = false;
          stopRaf();
          return;
        }
        if (snap.seq !== lastSeq) {
          lastSeq = snap.seq;
          options.onCursor({ x: snap.x, y: snap.y });
        }
      } catch {
        // browser preview / ACL
      } finally {
        peeking = false;
      }
    }
    rafId = requestAnimationFrame((t) => {
      void tick(t);
    });
  }

  function startRaf() {
    if (stopped || rafId) {
      return;
    }
    // Force one paint even if cursor hasn't moved (e.g. spotlight first show / re-enter monitor).
    lastSeq = -1;
    syncGateFps();
    rafId = requestAnimationFrame((t) => {
      void tick(t);
    });
  }

  function syncRafWithMonitor() {
    if (isActiveMonitor()) {
      startRaf();
      return;
    }
    stopRaf();
    options.onLeaveMonitor?.();
  }

  onMounted(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      myIndex = parseOverlayIndex(getCurrentWindow().label, options.labelPrefix);
    } catch {
      myIndex = 0;
    }

    try {
      const snap = await invoke<CursorHubSnapshot>("peek_cursor_hub");
      activeMonitor.value = snap.monitor;
      if (myIndex !== null && snap.monitor === myIndex) {
        lastSeq = snap.seq;
        options.onCursor({ x: snap.x, y: snap.y });
      }
    } catch {
      // browser preview
    }

    try {
      unlistenMonitor = await listen<{ index: number }>("app://cursor-monitor", (event) => {
        const index = Number(event.payload?.index);
        if (!Number.isFinite(index)) {
          return;
        }
        activeMonitor.value = index;
        syncRafWithMonitor();
      });
    } catch {
      // browser preview
    }

    syncRafWithMonitor();
  });

  onUnmounted(() => {
    stopped = true;
    stopRaf();
    void unlistenMonitor?.();
  });
}
