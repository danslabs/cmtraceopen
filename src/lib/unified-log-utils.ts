/**
 * Simplifies verbose macOS unified log messages into scannable summaries,
 * and categorizes them for filtering/visual badges.
 *
 * The Intune Agent preset captures IntuneMdmDaemon/Agent/CompanyPortal logs.
 * These are dominated by NSURLSession task lifecycle noise. Per HTTP
 * transaction there are ~10 log lines but only the summary line is useful.
 * This module classifies each message so the UI can filter noise by default.
 */

export type LogCategory =
  | "noise"    // network path events, task lifecycle, connection state — hide by default
  | "http"     // task summary with stats — the only useful HTTP line per transaction
  | "app";     // actual application-level messages (not NSURLSession)

export interface SimplifiedMessage {
  summary: string;
  category: LogCategory;
}

/** Parse a task summary dict like {transaction_duration_ms=88, response_status=200, ...} */
function parseTaskSummary(raw: string): Record<string, string> {
  const result: Record<string, string> = {};
  const match = raw.match(/\{([^}]+)\}/);
  if (!match) return result;
  for (const pair of match[1].split(",")) {
    const [k, v] = pair.split("=").map((s) => s.trim());
    if (k && v) result[k] = v;
  }
  return result;
}

/** Format bytes into human-readable size */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

/**
 * Simplify a unified log message into a scannable summary + category.
 * Returns null if no simplification is possible (show raw message as "app").
 */
export function simplifyMessage(
  message: string,
  process: string,
): SimplifiedMessage | null {
  // Only simplify known Intune processes
  if (
    process !== "IntuneMdmDaemon" &&
    process !== "IntuneMdmAgent" &&
    process !== "CompanyPortal"
  ) {
    return null;
  }

  const msg = message.trim();

  // ==========================================================================
  // NOISE: All of these are hidden by default. They're NSURLSession lifecycle
  // events that occur ~10x per HTTP transaction with zero diagnostic value.
  // ==========================================================================

  // Network path events (the noisiest — dozens per second)
  if (msg.includes("event: path:satisfied_change") ||
      msg.includes("event: path:unsatisfied") ||
      msg.includes("event: path:satisfied")) {
    return { summary: "Network path change", category: "noise" };
  }

  // Connection events
  if (msg.includes("event: client:connection_reused")) {
    return { summary: "Connection reused", category: "noise" };
  }
  if (msg.includes("event: client:connection_idle")) {
    return { summary: "Connection idle", category: "noise" };
  }

  // Task lifecycle: resuming
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ resuming/i)) {
    return { summary: "Task starting", category: "noise" };
  }

  // Task lifecycle: using connection
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ now using Connection/i)) {
    return { summary: "Using connection", category: "noise" };
  }

  // Task lifecycle: sent request (noise — the summary has the same info with more detail)
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ sent request/i)) {
    return { summary: "Sent request", category: "noise" };
  }

  // Task lifecycle: received response (noise — the summary has status + timing)
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ received response/i)) {
    return { summary: "Received response", category: "noise" };
  }

  // Task lifecycle: done using connection
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ done using Connection/i)) {
    return { summary: "Connection released", category: "noise" };
  }

  // Task lifecycle: response ended
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ response ended/i)) {
    return { summary: "Response complete", category: "noise" };
  }

  // Task lifecycle: finished
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ finished successfully/i)) {
    return { summary: "Task completed", category: "noise" };
  }

  // ==========================================================================
  // HTTP: Task summary — the ONE useful line per HTTP transaction.
  // Shows status, timing, bytes, and cache hit in one line.
  // ==========================================================================

  const taskSummary = msg.match(
    /Task <([A-F0-9-]+)>\.\d+ summary for task (success|failure)\s*(\{[^}]+\})/i,
  );
  if (taskSummary) {
    const outcome = taskSummary[2];
    const stats = parseTaskSummary(taskSummary[3]);
    const parts: string[] = [];

    // Status code
    const statusCode = stats.response_status ?? "???";
    const statusLabel = statusCode.startsWith("2")
      ? "OK"
      : statusCode.startsWith("3")
        ? "Redirect"
        : statusCode.startsWith("4")
          ? "Client Error"
          : statusCode.startsWith("5")
            ? "Server Error"
            : "";

    parts.push(`${statusCode} ${statusLabel}`);

    // Timing
    if (stats.transaction_duration_ms) {
      parts.push(`${stats.transaction_duration_ms}ms`);
    }

    // Bytes
    if (stats.request_bytes) {
      parts.push(`sent ${formatBytes(parseInt(stats.request_bytes, 10))}`);
    }
    if (stats.response_bytes) {
      parts.push(`recv ${formatBytes(parseInt(stats.response_bytes, 10))}`);
    }

    // Cache
    if (stats.cache_hit === "true") {
      parts.push("cached");
    }

    // Connection reuse
    if (stats.reused === "1" && stats.reused_after_ms) {
      const reusedSec = (parseInt(stats.reused_after_ms, 10) / 1000).toFixed(0);
      parts.push(`reused after ${reusedSec}s`);
    }

    const icon = outcome === "success" ? "MDM Request" : "MDM Request FAILED";
    return {
      summary: `${icon}: ${parts.join(" · ")}`,
      category: "http",
    };
  }

  // Task failure without summary block
  if (msg.match(/Task <[A-F0-9-]+>\.\d+ summary for task failure/i)) {
    return { summary: "MDM Request FAILED", category: "http" };
  }

  // ==========================================================================
  // Everything else is an actual application-level message — always show it.
  // ==========================================================================
  return null;
}
