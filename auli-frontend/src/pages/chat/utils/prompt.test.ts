import { describe, it, expect } from "vitest";
import { MIN_PROMPT_LENGTH, isPromptValid, charsRemaining } from "./prompt";

describe("isPromptValid", () => {
  it("rejects prompts shorter than the minimum", () => {
    expect(isPromptValid("")).toBe(false);
    expect(isPromptValid("short")).toBe(false);
    expect(isPromptValid("a".repeat(MIN_PROMPT_LENGTH - 1))).toBe(false);
  });

  it("accepts a prompt of exactly the minimum length", () => {
    expect(isPromptValid("a".repeat(MIN_PROMPT_LENGTH))).toBe(true);
  });

  it("ignores surrounding whitespace", () => {
    expect(isPromptValid(`   ${"a".repeat(MIN_PROMPT_LENGTH)}   `)).toBe(true);
    expect(isPromptValid("   short   ")).toBe(false);
  });
});

describe("charsRemaining", () => {
  it("counts characters still needed to reach the minimum", () => {
    expect(charsRemaining("")).toBe(MIN_PROMPT_LENGTH);
    expect(charsRemaining("abc")).toBe(MIN_PROMPT_LENGTH - 3);
  });

  it("never goes negative once the minimum is reached", () => {
    expect(charsRemaining("a".repeat(MIN_PROMPT_LENGTH))).toBe(0);
    expect(charsRemaining("a".repeat(MIN_PROMPT_LENGTH + 5))).toBe(0);
  });

  it("counts against the trimmed length", () => {
    expect(charsRemaining("  ab  ")).toBe(MIN_PROMPT_LENGTH - 2);
  });
});
