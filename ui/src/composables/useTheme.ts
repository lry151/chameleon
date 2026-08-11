import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { prefs } from "./usePrefs";
import { appState } from "./useAppState";
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
  });
  onUnmounted(() => {
    mqCleanup?.();
    mqCleanup = null;
  });

  const isDark = computed(() =>
    resolveIsDark(prefs.value.theme, systemIsDark.value),
  );

  /// 后端 detect_backdrop_capability() 是否给出真材质能力（Mica/Acrylic）。
  /// 深色模式下：capable → body 透明 + 半透面板（真玻璃）；None → 实色暗底 + 不透明面板。
  const backdropCapable = computed(() => appState.value.backdrop !== "None");

  const naiveTheme = computed(() => (isDark.value ? darkTheme : null));

  const fluentOverride = computed<GlobalThemeOverrides>(() =>
    buildFluentThemeOverride(
      prefs.value.accent_color,
      isDark.value,
      prefs.value.panel_opacity,
      backdropCapable.value,
    ),
  );
  // 外壳背景策略（自适应 backdrop 能力）：
  // - capable + 深色 → body 透明，DWM 真材质透出；topbar 轻微暗 tint；面板半透明（panel_opacity 驱动）。
  // - None 深色 → body 实色暗底（#16181A），面板不透明。绝不白屏。
  // - 浅色 → 全实色 #F3F3F3。
  watch(
    () => ({
      dark: isDark.value,
      opacity: prefs.value.panel_opacity,
      backdrop: backdropCapable.value,
    }),
    ({ dark, opacity, backdrop }) => {
      applyBodyBackground(dark, backdrop);
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
