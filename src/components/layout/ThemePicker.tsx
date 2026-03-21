import {
  Menu,
  MenuTrigger,
  MenuPopover,
  MenuList,
  MenuItem,
  MenuButton,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import { useUiStore } from "../../stores/ui-store";
import { getAllThemes, getThemeById } from "../../lib/themes";

const useStyles = makeStyles({
  swatch: {
    width: "14px",
    height: "14px",
    borderRadius: "50%",
    border: `1px solid ${tokens.colorNeutralStroke1}`,
    display: "inline-block",
    flexShrink: 0,
  },
  activeLabel: {
    color: tokens.colorBrandForeground1,
    fontWeight: 600,
  },
});

export function ThemePicker() {
  const styles = useStyles();
  const themeId = useUiStore((s) => s.themeId);
  const setThemeId = useUiStore((s) => s.setThemeId);
  const themes = getAllThemes();
  const activeTheme = getThemeById(themeId);

  return (
    <Menu>
      <MenuTrigger disableButtonEnhancement>
        <MenuButton
          size="small"
          appearance="subtle"
        >
          {activeTheme.label}
        </MenuButton>
      </MenuTrigger>
      <MenuPopover>
        <MenuList>
          {themes.map((theme) => (
            <MenuItem
              key={theme.id}
              onClick={() => setThemeId(theme.id)}
              icon={
                <div
                  className={styles.swatch}
                  style={{ backgroundColor: theme.swatchColor }}
                />
              }
            >
              <span className={theme.id === themeId ? styles.activeLabel : undefined}>
                {theme.label}
                {theme.id === themeId ? " \u2713" : ""}
              </span>
            </MenuItem>
          ))}
        </MenuList>
      </MenuPopover>
    </Menu>
  );
}
