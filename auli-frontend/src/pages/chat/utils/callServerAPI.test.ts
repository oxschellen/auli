import { describe, it, expect, vi, beforeEach } from "vitest";
import axios from "axios";
import { callServerAPI } from "./callServerAPI";
import type { Message } from "../../../types/chat";

vi.mock("axios", () => ({
  default: {
    post: vi.fn(),
    isCancel: vi.fn(() => false),
  },
}));

const mockedPost = vi.mocked(axios.post);
const mockedIsCancel = vi.mocked(axios.isCancel);

function makeArgs() {
  const setMessages = vi.fn();
  const setPrompt = vi.fn();
  const setLoading = vi.fn();
  const messages: Message[] = [
    { id: "greeting", from: "server", text: "Olá!", showButton: false },
  ];
  return {
    setMessages,
    setPrompt,
    setLoading,
    args: {
      prompt: "Uma pergunta longa o suficiente",
      messages,
      setMessages,
      setPrompt,
      setLoading,
      API_URL: "https://api.test/question",
      questionType: "1" as const,
    },
  };
}

/** The last array handed to setMessages reflects the final transcript. */
function finalMessages(setMessages: ReturnType<typeof vi.fn>): Message[] {
  return setMessages.mock.calls.at(-1)?.[0] as Message[];
}

beforeEach(() => {
  vi.clearAllMocks();
  mockedIsCancel.mockReturnValue(false);
});

describe("callServerAPI", () => {
  it("toggles loading on then off", async () => {
    mockedPost.mockResolvedValue({ data: { answer: "ok" } });
    const { setLoading, args } = makeArgs();

    await callServerAPI(args);

    expect(setLoading).toHaveBeenNthCalledWith(1, true);
    expect(setLoading).toHaveBeenLastCalledWith(false);
  });

  it("optimistically shows the user message and a thinking placeholder", async () => {
    mockedPost.mockResolvedValue({ data: { answer: "ok" } });
    const { setMessages, args } = makeArgs();

    await callServerAPI(args);

    const firstUpdate = setMessages.mock.calls[0][0] as Message[];
    expect(firstUpdate.map((m) => m.from)).toEqual(["server", "user", "server"]);
    expect(firstUpdate[1].text).toBe(args.prompt);
    expect(firstUpdate[2].text).toMatch(/pensando/i);
  });

  it("replaces the placeholder with the server answer and clears the prompt", async () => {
    mockedPost.mockResolvedValue({ data: { answer: "Resposta do servidor" } });
    const { setMessages, setPrompt, args } = makeArgs();

    await callServerAPI(args);

    const final = finalMessages(setMessages);
    const last = final[final.length - 1];
    expect(last).toMatchObject({ from: "server", text: "Resposta do servidor", showButton: true });
    expect(final).toHaveLength(3); // greeting + user + answer (placeholder removed)
    expect(setPrompt).toHaveBeenCalledWith("");
  });

  it("sends question, entity, and the selected type in the request body", async () => {
    mockedPost.mockResolvedValue({ data: { answer: "ok" } });
    const { args } = makeArgs();

    await callServerAPI({ ...args, entityId: "rs", questionType: "2" });

    expect(mockedPost).toHaveBeenCalledWith(
      args.API_URL,
      { question: args.prompt, entity: "rs", type: 2 },
      expect.anything(),
    );
  });

  it("falls back to a copy message when the answer is empty", async () => {
    mockedPost.mockResolvedValue({ data: {} });
    const { setMessages, args } = makeArgs();

    await callServerAPI(args);

    expect(finalMessages(setMessages).at(-1)?.text).toMatch(/obter resposta/i);
  });

  it("shows the timeout message when the request was cancelled", async () => {
    mockedPost.mockRejectedValue(new Error("aborted"));
    mockedIsCancel.mockReturnValue(true);
    const { setMessages, setPrompt, args } = makeArgs();

    await callServerAPI(args);

    expect(finalMessages(setMessages).at(-1)?.text).toMatch(/tempo limite/i);
    expect(setPrompt).toHaveBeenCalledWith("");
  });

  it("shows the unavailable message on a network error", async () => {
    mockedPost.mockRejectedValue(new Error("network"));
    mockedIsCancel.mockReturnValue(false);
    const { setMessages, args } = makeArgs();

    await callServerAPI(args);

    expect(finalMessages(setMessages).at(-1)?.text).toMatch(/não está disponível/i);
  });

  it("surfaces the server's rate-limit message on HTTP 429", async () => {
    mockedPost.mockRejectedValue({
      response: { status: 429, data: { error: "Muitas requisições. Aguarde." } },
    });
    mockedIsCancel.mockReturnValue(false);
    const { setMessages, args } = makeArgs();

    await callServerAPI(args);

    expect(finalMessages(setMessages).at(-1)?.text).toMatch(/muitas requisições/i);
  });
});
