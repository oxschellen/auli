import { useState } from "react";

/** The query type sent with a chat question. "1" = Serviços+FAQs (default), "2" = Pareceres. */
export type QuestionType = "1" | "2";

const STORAGE_KEY = "auli.questionType";
const DEFAULT_TYPE: QuestionType = "1";

function isQuestionType(value: string | null): value is QuestionType {
  return value === "1" || value === "2";
}

/** Reads the persisted type once at startup (falls back to the default if absent/unknown). */
function readStored(): QuestionType {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isQuestionType(stored) ? stored : DEFAULT_TYPE;
  } catch {
    return DEFAULT_TYPE;
  }
}

/** Selected chat query type, persisted globally (single localStorage key, shared across entities). */
export const useQuestionType = () => {
  const [questionType, setQuestionType] = useState<QuestionType>(readStored);

  // RadioGroup hands back `string | null`; validate before persisting/setting.
  const updateQuestionType = (value: string | null) => {
    if (!isQuestionType(value)) return;
    try {
      localStorage.setItem(STORAGE_KEY, value);
    } catch {
      // Non-fatal: selection still works for this session without persistence.
    }
    setQuestionType(value);
  };

  return { questionType, updateQuestionType };
};
