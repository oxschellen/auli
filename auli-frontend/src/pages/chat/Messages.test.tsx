// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProvider } from "../../test/render";
import { Messages } from "./Messages";
import type { Message } from "../../types/chat";

const messages: Message[] = [
  { id: "1", from: "server", text: "Olá! Como posso ajudar?", showButton: false },
  { id: "2", from: "user", text: "Qual a alíquota do ICMS?", showButton: true },
  { id: "3", from: "server", text: "Depende do produto.", showButton: true },
];

describe("Messages", () => {
  it("renders both user and server message text", () => {
    renderWithProvider(<Messages messages={messages} setPrompt={vi.fn()} />);
    expect(screen.getByText("Olá! Como posso ajudar?")).toBeInTheDocument();
    expect(screen.getByText("Qual a alíquota do ICMS?")).toBeInTheDocument();
    expect(screen.getByText("Depende do produto.")).toBeInTheDocument();
  });

  it("renders nothing for an empty transcript", () => {
    const { container } = renderWithProvider(<Messages messages={[]} setPrompt={vi.fn()} />);
    // The Flex wrapper renders but has no message children.
    expect(container.querySelector("p, .markdown-body")).toBeNull();
  });
});
