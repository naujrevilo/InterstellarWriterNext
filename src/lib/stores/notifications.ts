import { writable, derived } from "svelte/store";

export interface Notification {
  id: string;
  message: string;
  type: "success" | "error" | "info";
  timeout?: number;
}

function createNotificationStore() {
  const { subscribe, update } = writable<Notification[]>([]);

  return {
    subscribe,
    success(message: string, timeout = 3000) {
      const id = crypto.randomUUID();
      update((n) => [...n, { id, message, type: "success", timeout }]);
      if (timeout > 0) {
        setTimeout(() => this.dismiss(id), timeout);
      }
    },
    error(message: string, timeout = 5000) {
      const id = crypto.randomUUID();
      update((n) => [...n, { id, message, type: "error", timeout }]);
      if (timeout > 0) {
        setTimeout(() => this.dismiss(id), timeout);
      }
    },
    info(message: string, timeout = 3000) {
      const id = crypto.randomUUID();
      update((n) => [...n, { id, message, type: "info", timeout }]);
      if (timeout > 0) {
        setTimeout(() => this.dismiss(id), timeout);
      }
    },
    dismiss(id: string) {
      update((n) => n.filter((item) => item.id !== id));
    },
  };
}

export const notifications = createNotificationStore();
