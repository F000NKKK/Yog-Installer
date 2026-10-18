import { WindowControls, useT } from "substrate-platform-ui";
import { Notifications } from "./Notifications";

/** The window's own title bar: its name, the notifications, the window buttons. */
export function TitleBar({ notifications }: { notifications: string[] }) {
  const t = useT();
  return (
    <header className="yi-titlebar" data-tauri-drag-region>
      <span className="yi-titlebar-name" data-tauri-drag-region>
        {t("Yog Installer")}
      </span>
      <div className="yi-titlebar-actions">
        <Notifications messages={notifications} />
        <WindowControls />
      </div>
    </header>
  );
}
