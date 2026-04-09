/**
 * Toast Store — Global notification system.
 *
 * Any view can push transient messages (success, error, info).
 * Toast.vue renders them as a stacked bottom-right overlay.
 */
import { ref } from "vue";

export interface ToastMessage {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

const toasts = ref<ToastMessage[]>([]);
let nextId = 0;

export function showToast(message: string, type: "success" | "error" | "info" = "success") {
  toasts.value.push({ id: nextId++, message, type });
}

export function dismissToast(id: number) {
  toasts.value = toasts.value.filter(t => t.id !== id);
}

export { toasts };
