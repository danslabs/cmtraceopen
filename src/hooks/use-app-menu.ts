import { useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppActions } from "../components/layout/Toolbar";
import { collectDiagnostics } from "../lib/commands";
import { loadPathAsLogSource } from "../lib/log-source";
import { useUiStore } from "../stores/ui-store";

const MENU_EVENT_APP_ACTION = "app-menu-action";

interface AppMenuActionPayload {
  version: number;
  menu_id: string;
  action: string;
  category: string;
  trigger: string;
  source_id: string | null;
}

export function useAppMenu() {
  const {
    openSourceFileDialog,
    openSourceFolderDialog,
    openKnownSourceCatalogAction,
    showFindBar,
    showFilterDialog,
    showErrorLookupDialog,
    showEvidenceBundleDialog,
    showAboutDialog,
    showAccessibilityDialog,
    togglePauseResume,
    refreshActiveSource,
    toggleDetailsPane,
    toggleInfoPane,
  } = useAppActions();

  const collectingRef = useRef(false);

  const handleCollectDiagnostics = useCallback(async () => {
    if (collectingRef.current) {
      console.warn("[app-menu] diagnostics collection already in progress");
      return;
    }

    collectingRef.current = true;
    const setCollectionProgress = useUiStore.getState().setCollectionProgress;

    const requestId = `collect-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setCollectionProgress({ requestId, message: "Starting collection...", completedItems: 0, totalItems: 0, currentItem: null });

    try {
      const result = await collectDiagnostics(requestId);
      setCollectionProgress(null);
      console.info("[app-menu] diagnostics collection complete", result);

      // Auto-open the bundle in the log workspace.
      if (result.bundlePath) {
        await loadPathAsLogSource(result.bundlePath, { preferFolder: true });
      }
    } catch (error) {
      setCollectionProgress(null);
      console.error("[app-menu] diagnostics collection failed", { error });
    } finally {
      collectingRef.current = false;
    }
  }, []);

  useEffect(() => {
    let disposed = false;

    const handleAction = async (payload: AppMenuActionPayload) => {
      if (disposed) {
        return;
      }

      console.info("[app-menu] handling native menu action", { payload });

      try {
        switch (payload.action) {
          case "open_log_file_dialog":
            await openSourceFileDialog();
            return;
          case "open_log_folder_dialog":
            await openSourceFolderDialog();
            return;
          case "show_find":
            showFindBar();
            return;
          case "show_filter":
            showFilterDialog();
            return;
          case "show_error_lookup":
            showErrorLookupDialog();
            return;
          case "show_evidence_bundle":
            showEvidenceBundleDialog();
            return;
          case "toggle_pause":
            togglePauseResume();
            return;
          case "refresh":
            await refreshActiveSource();
            return;
          case "toggle_details":
            toggleDetailsPane();
            return;
          case "toggle_info_pane":
            toggleInfoPane();
            return;
          case "show_about":
            showAboutDialog();
            return;
          case "show_accessibility_settings":
            showAccessibilityDialog();
            return;
          case "collect_diagnostics":
            await handleCollectDiagnostics();
            return;
          case "open_known_source": {
            if (payload.source_id) {
              await openKnownSourceCatalogAction({
                sourceId: payload.source_id,
                trigger: payload.trigger || "native-menu.known-source",
              });
            } else {
              console.warn("[app-menu] open_known_source received without source_id", { payload });
            }
            return;
          }
          default:
            console.warn("[app-menu] unhandled native menu action", { payload });
        }
      } catch (error) {
        console.error("[app-menu] failed to handle native menu action", {
          payload,
          error,
        });
      }
    };

    const unlistenActionPromise = listen<AppMenuActionPayload>(
      MENU_EVENT_APP_ACTION,
      async (event) => {
        await handleAction(event.payload);
      }
    );

    return () => {
      disposed = true;

      unlistenActionPromise
        .then((unlisten) => unlisten())
        .catch((error) => {
          console.error("[app-menu] failed to clean up menu action listener", {
            error,
          });
        });
    };
  }, [
    handleCollectDiagnostics,
    openKnownSourceCatalogAction,
    openSourceFileDialog,
    openSourceFolderDialog,
    refreshActiveSource,
    showAccessibilityDialog,
    showAboutDialog,
    showErrorLookupDialog,
    showEvidenceBundleDialog,
    showFilterDialog,
    showFindBar,
    toggleDetailsPane,
    toggleInfoPane,
    togglePauseResume,
  ]);
}
