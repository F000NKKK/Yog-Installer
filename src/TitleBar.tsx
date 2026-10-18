import { Icon, IconButton, WindowControls, useT } from "substrate-platform-ui";

/** The window's own title bar: its name, the notifications bell, the window buttons. */
export function TitleBar({ notifications, onNotifications }: { notifications: number; onNotifications: () => void }) {
  const t = useT();
  return (
    <header className="yi-titlebar" data-tauri-drag-region>
      <span className="yi-titlebar-name" data-tauri-drag-region>
        {t("Yog Installer")}
      </span>
      <div className="yi-titlebar-actions">
        <span className="yi-bell">
          <IconButton title={t("Notifications")} onClick={onNotifications}>
            <Icon name="bell" />
          </IconButton>
          {notifications > 0 && <span className="yi-badge">{notifications}</span>}
        </span>
        <WindowControls />
      </div>
    </header>
  );
}
