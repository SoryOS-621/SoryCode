import React from "react";
import { createRoot } from "react-dom/client";
import "@sorycode/ui/styles.css";
import "../styles/app.css";
import { AppProviders } from "./providers/AppProviders";
import { App } from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("missing #root element");

createRoot(root).render(
  <React.StrictMode>
    <AppProviders>
      <App />
    </AppProviders>
  </React.StrictMode>,
);
