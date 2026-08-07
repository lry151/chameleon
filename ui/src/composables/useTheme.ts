import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { prefs } from "./usePrefs";
import { applyBodyBackground, resolveIsDark } from "../theme/hybrid";
import { buildFluentThemeOverride } from "../theme/fluent";
import { type GlobalThemeOverrides, darkTheme } from "naive-ui";

/// 系统深色状态（仅 System 模式下使用）。
const systemIsDark = ref(
  typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: dark)").matches === true,
);

let mqCleanup: (() => void) | null = null;

function startSystemWatcher(): void {
  if (typeof window === "undefined" || !window.matchMedia) return;
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) => {
    systemIsDark.value = e.matches;
  };
  mq.addEventListener("change", handler);
  mqCleanup = () => mq.removeEventListener("change", handler);
}

/// 组合式：响应式 isDark / 当前 theme（Naive UI）/ Fluent theme overrides。
/// App.vue 挂载一次即可驱动全局。
export function useTheme() {
  onMounted(() => {
    startSystemWatcher();
    // 启动时立即同步一次 body 背景。
    applyBodyBackground(isDark.value);
  });
  onUnmounted(() => {
    mqCleanup?.();
    mqCleanup = null;
  });

  const isDark = computed(() =>
    resolveIsDark(prefs.value.theme, systemIsDark.value),
  );

  const naiveTheme = computed(() => (isDark.value ? darkTheme : null));

  const fluentOverride = computed<GlobalThemeOverrides>(() =>
    buildFluentThemeOverride(prefs.value.accent_color),
  );

  // 外壳背景策略：深色让 Mica 透出 + Topbar 轻微暗 tint；浅色全实色。
  // panel_opacity 控制深色 tint 强度（0.5–1.0）。
  watch(
    () => ({ dark: isDark.value, opacity: prefs.value.panel_opacity }),
    ({ dark, opacity }) => {
      const root = document.documentElement.style;
      if (dark) {
        root.setProperty(
          "--topbar-bg",
          `rgba(0, 0, 0, ${(0.25 * opacity).toFixed(3)})`,
        );
        root.setProperty("--main-bg", "transparent");
      } else {
        root.setProperty("--topbar-bg", "rgba(255, 255, 255, 0.75)");
        root.setProperty("--main-bg", "#F3F3F3");
      }
    },
    { immediate: true },
  );
  return { isDark, naiveTheme, fluentOverride };
}
