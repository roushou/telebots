import { HeadContent, Outlet, Scripts, createRootRoute } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Toaster } from "sonner";

import { TooltipProvider } from "../components/ui/tooltip";
import { ThemeProvider, useTheme } from "../lib/theme";

import appCss from "../index.css?url";

/// Set the theme class before hydration so the first paint never flashes the
/// wrong mode. Default is dark; a stored "light" wins.
const themeScript = `(function(){try{var t=localStorage.getItem("theme");var d=t!=="light";document.documentElement.classList.toggle("dark",d);}catch(e){document.documentElement.classList.add("dark");}})();`;

function ThemedToaster() {
  const { theme } = useTheme();
  return <Toaster position="bottom-right" theme={theme} richColors />;
}

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1" },
      { title: "Telebots Monitor" },
    ],
    links: [{ rel: "stylesheet", href: appCss }],
  }),
  component: RootComponent,
});

function RootComponent() {
  return (
    <RootDocument>
      <ThemeProvider>
        <TooltipProvider delayDuration={150}>
          <Outlet />
        </TooltipProvider>
        <ThemedToaster />
      </ThemeProvider>
    </RootDocument>
  );
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}
