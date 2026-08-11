import type { GlobalThemeOverrides } from "naive-ui";

const FONT_FAMILY =
  '"Microsoft YaHei", "PingFang SC", "Segoe UI", system-ui, -apple-system, sans-serif';

/// Fluent Design tokens → Naive UI 主题覆盖。
/// accent: 用户可选的主色（默认 #1abc9c），扩散到 primaryColor 系列。
/// isDark + panelOpacity + backdropCapable：深色且后端检测到 Mica/Acrylic 可用时，
/// 把面板(Card)背景设为半透明暗色，让「面板透明度」滑块真正驱动玻璃效果、
/// 且能透出 DWM 真材质。None 能力下保持实色（面板不透明，避免露白）。
export function buildFluentThemeOverride(
  accent: string,
  isDark: boolean,
  panelOpacity: number,
  backdropCapable: boolean,
): GlobalThemeOverrides {
  return {
    common: {
      fontFamily: FONT_FAMILY,
      fontSize: "14px",
      borderRadius: "4px",

      primaryColor: accent,
      primaryColorHover: shiftLightness(accent, +10),
      primaryColorPressed: shiftLightness(accent, -8),
      primaryColorSuppl: shiftLightness(accent, +14),
    },
    Button: {
      borderRadiusMedium: "999px",
      borderRadiusSmall: "999px",
      borderRadiusLarge: "999px",
      fontWeight: "600",
      textColorFocus: accent,
    },
    Card: {
      borderRadius: "12px",
      ...(isDark && backdropCapable
        ? {
            color: `rgba(31, 31, 34, ${panelOpacity})`,
            colorModal: `rgba(31, 31, 34, ${panelOpacity})`,
            colorPopover: `rgba(31, 31, 34, ${panelOpacity})`,
          }
        : {}),
    },
    Dialog: {
      borderRadius: "12px",
    },
  };
}

/// 简单 hex lightness shift：±N 单位（-100..100）。
function shiftLightness(hex: string, delta: number): string {
  const c = hex.replace("#", "");
  if (c.length !== 6) return hex;
  const r = parseInt(c.slice(0, 2), 16);
  const g = parseInt(c.slice(2, 4), 16);
  const b = parseInt(c.slice(4, 6), 16);
  const f = delta / 100;
  const adj = (v: number) =>
    Math.max(0, Math.min(255, Math.round(v + (f > 0 ? (255 - v) * f : v * f))));
  const toHex = (v: number) => v.toString(16).padStart(2, "0");
  return `#${toHex(adj(r))}${toHex(adj(g))}${toHex(adj(b))}`;
}
