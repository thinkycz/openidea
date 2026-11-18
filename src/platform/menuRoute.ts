export const MENU_EVENT = "openidea://menu";

export interface MenuRoute {
  action: string;
  arg?: string | null;
  /** The window in front when the item was chosen, if it is not this one (this window's Settings). */
  returnTo?: string;
}

export function isMenuRoute(v: unknown): v is MenuRoute {
  return typeof v === "object" && v !== null && typeof (v as MenuRoute).action === "string";
}
