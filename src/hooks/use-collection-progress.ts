import { useEffect, useState, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { collectDiagnostics, type CollectionResult } from "../lib/commands";

const COLLECTION_PROGRESS_EVENT = "collection-progress";

export interface CollectionProgressPayload {
  requestId: string;
  message: string;
  currentItem: string | null;
  completedItems: number;
  totalItems: number;
}

export interface CollectionState {
  isCollecting: boolean;
  progress: CollectionProgressPayload | null;
  result: CollectionResult | null;
  error: string | null;
}

export function useCollectionProgress() {
  const [state, setState] = useState<CollectionState>({
    isCollecting: false,
    progress: null,
    result: null,
    error: null,
  });

  const requestIdRef = useRef<string | null>(null);

  useEffect(() => {
    const unlisten = listen<CollectionProgressPayload>(
      COLLECTION_PROGRESS_EVENT,
      (event) => {
        if (
          requestIdRef.current &&
          event.payload.requestId === requestIdRef.current
        ) {
          setState((prev) => ({ ...prev, progress: event.payload }));
        }
      }
    );

    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, []);

  const startCollection = useCallback(async (): Promise<CollectionResult | null> => {
    const requestId = `collect-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    requestIdRef.current = requestId;

    setState({
      isCollecting: true,
      progress: null,
      result: null,
      error: null,
    });

    try {
      const result = await collectDiagnostics(requestId);
      setState({
        isCollecting: false,
        progress: null,
        result,
        error: null,
      });
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setState({
        isCollecting: false,
        progress: null,
        result: null,
        error: message,
      });
      return null;
    } finally {
      requestIdRef.current = null;
    }
  }, []);

  return { ...state, startCollection };
}
