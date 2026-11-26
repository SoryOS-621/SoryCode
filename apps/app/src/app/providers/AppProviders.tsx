import type { ReactNode } from "react";
import { ThemeProvider } from "@sorycode/ui";
import { EngineProvider } from "./EngineProvider";

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider>
      <EngineProvider>{children}</EngineProvider>
    </ThemeProvider>
  );
}
