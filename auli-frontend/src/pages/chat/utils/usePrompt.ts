import { useState } from "react";
import type { ChangeEvent } from "react";

export const usePrompt = () => {
  const [prompt, setPrompt] = useState("");

  const updatePrompt = (e: ChangeEvent<HTMLTextAreaElement>) => {
    setPrompt(e.target.value);
  };

  return { prompt, setPrompt, updatePrompt };
};
