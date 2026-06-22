/** Minimum length (after trimming) a question must reach before it can be sent. */
export const MIN_PROMPT_LENGTH = 10;

/** Whether a prompt is long enough to submit. Single source of truth for the
 *  send guard, the button's disabled state, and the character-count hint —
 *  previously these used inconsistent comparisons (`<= 10` vs `< 10`), so a
 *  10-character prompt was simultaneously "ready" (hint) and rejected (send). */
export const isPromptValid = (text: string): boolean =>
  text.trim().length >= MIN_PROMPT_LENGTH;

/** Characters still needed to reach the minimum (0 once valid). */
export const charsRemaining = (text: string): number =>
  Math.max(0, MIN_PROMPT_LENGTH - text.trim().length);
