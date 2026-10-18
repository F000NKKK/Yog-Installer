import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "substrate-platform-ui";
import App from "./App";
import "./locales/ru";

// The webview's own menu (Back / Reload / Inspect) isn't the Installer's.
document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement | null)?.closest("input, textarea")) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);
