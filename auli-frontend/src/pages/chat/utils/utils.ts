import { toaster } from "../ui/toaster";

const FAILURE_MESSAGE =
  "Não foi possível copiar o texto para a área de transferência.";

const showToast = (description: string): void => {
  toaster.create({ description, type: "info", duration: 1000, closable: false });
};

export const utilsCopyTextToClipboard = (
  text: string,
  description: string,
): void => {
  // `navigator.clipboard` is undefined on insecure origins; calling it there
  // would throw synchronously and skip the catch below.
  if (!navigator.clipboard) {
    showToast(FAILURE_MESSAGE);
    return;
  }

  navigator.clipboard
    .writeText(text)
    .then(() => showToast(description))
    .catch((error: unknown) => {
      console.error("Error copying text to clipboard:", error);
      showToast(FAILURE_MESSAGE);
    });
};
