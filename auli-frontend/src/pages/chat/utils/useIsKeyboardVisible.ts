// useIsMobileKeyboardVisible.jsx
import { useSyncExternalStore } from "react";

function getKeyboardHeight() {
  const viewport = window.visualViewport;
  const layoutHeight = window.innerHeight;

  const height = viewport
    ? Math.max(0, layoutHeight - viewport.height - viewport.offsetTop)
    : Math.max(0, screen.height - layoutHeight);

  // Treat anything under the threshold as "keyboard closed".
  return height > 120 ? height : 0;
}

function subscribe(callback: () => void) {
  window.addEventListener("resize", callback);
  window.visualViewport?.addEventListener("resize", callback);
  return () => {
    window.removeEventListener("resize", callback);
    window.visualViewport?.removeEventListener("resize", callback);
  };
}

export const useIsKeyboardVisible = () => {
  // getSnapshot returns a primitive (number), so it stays referentially stable.
  const keyboardHeight = useSyncExternalStore(subscribe, getKeyboardHeight, () => 0);

  return { isKeyboardVisible: keyboardHeight > 0, keyboardHeight };
};
