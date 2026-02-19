// T-APP-001.3: Host theme integration via @modelcontextprotocol/ext-apps
import { applyDocumentTheme, applyHostStyleVariables, applyHostFonts } from "@modelcontextprotocol/ext-apps";

export const COMMUNITY_COLORS = [
  "#4A90D9", "#D94A4A", "#4AD9A0", "#D9A04A",
  "#9B59B6", "#2ECC71", "#E67E22", "#1ABC9C",
  "#E74C3C", "#3498DB", "#F39C12", "#8E44AD"
];

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function applyTheme(ctx: any): void {
  if (ctx.theme) applyDocumentTheme(ctx.theme);
  if (ctx.styles?.variables) applyHostStyleVariables(ctx.styles.variables);
  if (ctx.styles?.css?.fonts) applyHostFonts(ctx.styles.css.fonts);
  if (ctx.safeAreaInsets) {
    const { top, right, bottom, left } = ctx.safeAreaInsets;
    document.body.style.padding = `${top}px ${right}px ${bottom}px ${left}px`;
  }
}

export function getCommunityColor(communityId: number | null): string {
  if (communityId === null) return "#888";
  return COMMUNITY_COLORS[communityId % COMMUNITY_COLORS.length];
}
