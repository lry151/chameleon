import { h } from "vue";
import type { MessageApi } from "naive-ui";

/// 显示后端错误文案：naive message 默认折叠空白，多行「提示/处理措施」会粘成一段。
/// 这里用 pre-line 容器按换行渲染，保持可读结构。
export function notifyError(
  message: MessageApi,
  text: string,
): void {
  message.error(() =>
    h("div", { style: "white-space: pre-line; line-height: 1.5" }, text),
  );
}

/// 从 Tauri invoke 的 rejection（string 或 Error）取出文案，加前缀后展示。
export function notifyBackendError(
  message: MessageApi,
  err: unknown,
  prefix: string,
): void {
  const detail =
    typeof err === "string"
      ? err
      : (err as { message?: unknown })?.message
        ? String((err as { message?: unknown }).message)
        : String(err);
  notifyError(message, `${prefix}：${detail}`);
}
