import { useEffect } from "react";

export function useKeyboardCtrlIntercept(
  key: string,
  reaction: (event: KeyboardEvent) => unknown
) {
  useEffect(() => {
    const wrappedReaction: typeof reaction = (event) => {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key.toLowerCase() !== key.toLowerCase()) return;
      event.preventDefault();
      reaction(event);
    };
    const controller = new AbortController();
    window.addEventListener("keydown", wrappedReaction, {
      signal: controller.signal,
    });

    return () => controller.abort();
  }, [key, reaction]);
}
