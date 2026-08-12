import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
} from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Toaster } from "sonner";
import { TooltipProvider } from "../components/ui/tooltip";
import { ThemeProvider, useTheme } from "../lib/theme";
import appCss from "../index.css?url";

/// Set the theme class before hydration so the first paint never flashes the
/// wrong mode. Default is dark; a stored "light" wins.
const themeScript = `(function(){try{var t=localStorage.getItem("theme");var d=t!=="light";document.documentElement.classList.toggle("dark",d);}catch(e){document.documentElement.classList.add("dark");}})();`;

/// Direction contract, emitted as a real HTML comment so it survives the build.
const contract = `
THESIS: one glanceable ops board — health at a glance, every red state traces
to an error and a point on the timeline; refuses hero-metric theater.
OWN-WORLD: shadcn/ui canon played straight — zinc neutrals, one primary accent,
semantic emerald/destructive status, system sans, 1px borders, soft shadows,
TanStack Charts SVG, lucide icons.
STORY: open the board, see who is down in seconds; click a bot to see when it
broke, when it restarted, and the error behind it.
FIRST VIEWPORT: sidebar (brand, Overview, per-bot status dots), header (theme
toggle, ⌘K), four stat cards, 24h health strip, bot grid.
FORM: category standard (shadcn dashboard canon), user-pinned.
FINISH: unreviewed and undocumented is unfinished; this build ends with the
finish review, the verdict, and DESIGN.md.
`;

function ContractComment() {
  return (
    <span
      aria-hidden="true"
      style={{ display: "none" }}
      suppressHydrationWarning
      dangerouslySetInnerHTML={{ __html: `<!--${contract}-->` }}
    />
  );
}

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
        <ContractComment />
        {children}
        <Scripts />
      </body>
    </html>
  );
}
