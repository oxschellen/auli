import type { Dispatch, SetStateAction } from "react";

/** Who authored a chat message. */
export type MessageSender = "user" | "server";

/** A single message rendered in the chat transcript. */
export interface Message {
  id: string;
  from: MessageSender;
  text: string;
  showButton: boolean;
}

/** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
export type SetPrompt = Dispatch<SetStateAction<string>>;
