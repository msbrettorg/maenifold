import { useEffect, useMemo, useState } from "react";
import { Command } from "cmdk";
import {
  MoonStar,
  RefreshCw,
  Search,
  SunMedium,
  Waypoints,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { type Theme } from "@/lib/theme";

type TopBarProps = {
  pipelineName: string | null;
  pipelineId: string | null;
  theme: Theme;
  onRefresh: () => Promise<void>;
  toggleTheme: () => void;
};

type CommandAction = {
  id: string;
  label: string;
  run: () => void;
};

export function TopBar({
  pipelineName,
  pipelineId,
  theme,
  onRefresh,
  toggleTheme,
}: TopBarProps) {
  const [open, setOpen] = useState(false);
  const actions = useMemo<CommandAction[]>(
    () => [
      {
        id: "refresh",
        label: "Refresh pipeline data",
        run: () => {
          void onRefresh();
          setOpen(false);
        },
      },
      {
        id: "theme",
        label: theme === "dark" ? "Switch to light theme" : "Switch to dark theme",
        run: () => {
          toggleTheme();
          setOpen(false);
        },
      },
      ...(pipelineId
        ? [
            {
              id: "copy-pipeline",
              label: `Copy pipeline id (${pipelineId})`,
              run: () => {
                void navigator.clipboard.writeText(pipelineId);
                setOpen(false);
              },
            },
          ]
        : []),
    ],
    [onRefresh, pipelineId, theme, toggleTheme],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setOpen((current) => !current);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <>
      <div className="flex h-11 items-center justify-between gap-3 border-b border-border px-4">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex size-7 items-center justify-center rounded-lg border border-primary/30 bg-primary/10 text-primary">
            <Waypoints className="size-4" />
          </div>
          <div className="min-w-0">
            <div className="text-sm font-semibold tracking-tight">Pipeline</div>
            <div className="truncate font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
              console / pipeline / {pipelineName ?? "active run"}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Badge variant="outline" className="font-mono text-[11px] uppercase tracking-[0.16em]">
            Active only
          </Badge>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={() => void onRefresh()}
            aria-label="Refresh pipeline data"
          >
            <RefreshCw className="size-4" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={toggleTheme}
            aria-label="Toggle theme"
          >
            {theme === "dark" ? <SunMedium className="size-4" /> : <MoonStar className="size-4" />}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setOpen(true)}
          >
            <Search className="size-4" />
            Cmd-K
          </Button>
        </div>
      </div>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side="right" className="w-[24rem] sm:max-w-[24rem]">
          <SheetHeader>
            <SheetTitle>Command palette</SheetTitle>
            <SheetDescription>Quick actions for the Pipeline screen.</SheetDescription>
          </SheetHeader>
          <div className="px-4 pb-4">
            <Command className="overflow-hidden rounded-xl border border-border bg-background">
              <Command.Input
                placeholder="Type a command"
                className="w-full border-b border-border bg-transparent px-4 py-3 text-sm outline-none"
              />
              <Command.List className="max-h-[24rem] overflow-auto p-2">
                <Command.Empty className="px-3 py-8 text-center text-sm text-muted-foreground">
                  No command matched.
                </Command.Empty>
                <Command.Group heading="Screen actions" className="space-y-1">
                  {actions.map((action) => (
                    <Command.Item
                      key={action.id}
                      value={action.label}
                      onSelect={action.run}
                      className="rounded-lg px-3 py-2 text-sm outline-none data-[selected=true]:bg-muted"
                    >
                      {action.label}
                    </Command.Item>
                  ))}
                </Command.Group>
              </Command.List>
            </Command>
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}
