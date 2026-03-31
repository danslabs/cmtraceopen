# Nested Known Log Sources Menu + ConfigMgr Sources

**Date**: 2026-03-31
**Issue**: #75

## Problem

The "Open Known Log Source" toolbar menu renders as a long flat list with `MenuGroupHeader` dividers. With ~12 groups and ~18 sources on Windows (growing), the menu is unwieldy. Additionally, ConfigMgr (SCCM/MECM) client log paths are missing from the catalog.

## Design

### Nested Submenu Structure

Replace the flat `MenuGroup`/`MenuGroupHeader` layout with two levels of Fluent UI `<Menu>` nesting:

- **Level 1 (top-level)**: Family names as submenu triggers (e.g., "Windows Intune >", "ConfigMgr >")
- **Level 2**: Group names as submenu triggers (e.g., "Intune IME >", "ConfigMgr Logs >")
- **Level 3**: Individual source items (e.g., "IntuneManagementExtension.log")

```
Top-level menu
├── Windows Intune >
│   ├── Intune IME >
│   │   ├── Intune IME Logs Folder
│   │   ├── IntuneManagementExtension.log
│   │   ├── AppWorkload.log
│   │   └── AgentExecutor.log
│   └── MDM and Enrollment >
│       └── DMClient Local Logs
├── ConfigMgr >
│   └── ConfigMgr Logs >
│       ├── CCM Logs Folder
│       ├── ccmsetup Logs Folder
│       ├── CCM Client Setup Logs
│       └── Software Metering Logs
├── Windows Setup >
│   └── Panther >
│       ├── setupact.log
│       └── setuperr.log
├── Windows Servicing >
│   ├── CBS and DISM >
│   │   ├── CBS.log
│   │   └── DISM.log
│   └── Windows Update >
│       └── ReportingEvents.log
├── Windows IIS >
│   └── W3C Logs >
│       └── IIS Logs
├── Software Deployment >
│   ├── Deployment Logs >
│   │   ├── Software Logs Folder
│   │   └── ccmcache Folder
│   ├── PSADT >
│   │   └── PSADT Logs
│   ├── MSI Logs >
│   │   └── MSI Log (Temp)
│   └── PatchMyPC >
│       ├── PatchMyPC Logs Folder
│       └── PatchMyPC Install Logs
```

macOS sources follow the same pattern with their own families (macOS Intune, macOS System, macOS Defender).

### Fluent UI Implementation

Fluent UI v9 supports nested menus via `<Menu>` inside `<MenuItem>`:

```tsx
<Menu>
  <MenuTrigger><MenuItem>Windows Intune</MenuItem></MenuTrigger>
  <MenuPopover>
    <MenuList>
      <Menu>
        <MenuTrigger><MenuItem>Intune IME</MenuItem></MenuTrigger>
        <MenuPopover>
          <MenuList>
            <MenuItem onClick={...}>Intune IME Logs Folder</MenuItem>
            <MenuItem onClick={...}>IntuneManagementExtension.log</MenuItem>
          </MenuList>
        </MenuPopover>
      </Menu>
    </MenuList>
  </MenuPopover>
</Menu>
```

### New ConfigMgr Sources (Backend)

Added to the Windows catalog in `known_sources.rs`:

| Field | Value |
|-------|-------|
| **Family** | `windows-configmgr`, label: "ConfigMgr", group_order: 20 |
| **Group** | `configmgr-logs`, label: "ConfigMgr Logs" |

Sources:

| ID | Label | Path | Kind |
|----|-------|------|------|
| `windows-configmgr-ccm-logs` | CCM Logs Folder | `C:\Windows\CCM\Logs` | Folder |
| `windows-configmgr-ccmsetup-logs` | ccmsetup Logs Folder | `C:\Windows\ccmsetup\Logs` | Folder |
| `windows-configmgr-setup-temp-logs` | CCM Client Setup Logs | `C:\Windows\Temp\CCMSetup\Logs` | Folder |
| `windows-configmgr-swmtr` | Software Metering Logs | `C:\Windows\System32\SWMTRReporting` | Folder |

All are Windows-only, folder-type sources with `openAllFiles` default behavior.

## Data Model Changes

### New type: `KnownSourceToolbarFamily`

```typescript
export interface KnownSourceToolbarFamily {
  id: string;          // familyId
  label: string;       // familyLabel
  sortOrder: number;   // groupOrder from first group in family
  groups: KnownSourceToolbarGroup[];
}
```

### Updated `KnownSourceToolbarGroup`

No changes needed — already has `id`, `label`, `sortOrder`, `sources[]`.

### Updated `buildToolbarKnownSourceGroups()`

Renamed to `buildToolbarKnownSourceFamilies()`. Returns `KnownSourceToolbarFamily[]` instead of `KnownSourceToolbarGroup[]`. Groups sources by familyId first, then by groupId within each family. Sorting: families by their lowest group_order, groups by group_order within family.

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/commands/known_sources.rs` | Add 4 ConfigMgr sources |
| `src/types/log.ts` | Add `KnownSourceToolbarFamily` interface |
| `src/stores/log-store.ts` | Update grouping to produce family→group→source hierarchy |
| `src/components/layout/Toolbar.tsx` | Render nested `<Menu>` submenus |

## Testing

- Verify all families appear as top-level submenu triggers
- Verify groups appear as second-level submenu triggers within each family
- Verify clicking a source item opens the correct path
- Verify macOS sources render with their own family hierarchy
- Verify the menu is disabled when appropriate (busy, no sources)
