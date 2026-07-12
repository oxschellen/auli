// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useQuestionType } from "./useQuestionType";

const STORAGE_KEY = "auli.questionType";

beforeEach(() => {
  localStorage.clear();
});

describe("useQuestionType", () => {
  it("defaults to '1' when nothing is stored", () => {
    const { result } = renderHook(() => useQuestionType());
    expect(result.current.questionType).toBe("1");
  });

  it("persists the selection to localStorage", () => {
    const { result } = renderHook(() => useQuestionType());
    act(() => result.current.updateQuestionType("2"));
    expect(result.current.questionType).toBe("2");
    expect(localStorage.getItem(STORAGE_KEY)).toBe("2");
  });

  it("reads the persisted selection on init", () => {
    localStorage.setItem(STORAGE_KEY, "2");
    const { result } = renderHook(() => useQuestionType());
    expect(result.current.questionType).toBe("2");
  });

  it("ignores an unknown stored value and falls back to the default", () => {
    localStorage.setItem(STORAGE_KEY, "9");
    const { result } = renderHook(() => useQuestionType());
    expect(result.current.questionType).toBe("1");
  });

  it("ignores an invalid update (no state change, nothing persisted)", () => {
    const { result } = renderHook(() => useQuestionType());
    act(() => result.current.updateQuestionType("9"));
    expect(result.current.questionType).toBe("1");
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});
