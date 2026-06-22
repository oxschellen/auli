// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithProvider } from "../test/render";
import { SearchInput } from "./SearchInput";

const noop = () => {};

describe("SearchInput", () => {
  it("renders the input and no clear button when empty", () => {
    renderWithProvider(
      <SearchInput value="" onChange={noop} onClear={noop} placeholder="Pesquisar" />,
    );
    expect(screen.getByPlaceholderText("Pesquisar")).toBeInTheDocument();
    expect(screen.queryByLabelText("Limpar busca")).not.toBeInTheDocument();
  });

  it("calls onChange when the user types", () => {
    const onChange = vi.fn();
    renderWithProvider(
      <SearchInput value="" onChange={onChange} onClear={noop} placeholder="Pesquisar" />,
    );
    fireEvent.change(screen.getByPlaceholderText("Pesquisar"), {
      target: { value: "icms" },
    });
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("shows the clear button when there's a value and fires onClear", () => {
    const onClear = vi.fn();
    renderWithProvider(
      <SearchInput value="icms" onChange={noop} onClear={onClear} placeholder="Pesquisar" />,
    );
    fireEvent.click(screen.getByLabelText("Limpar busca"));
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("defaults the input's accessible name to the placeholder", () => {
    renderWithProvider(
      <SearchInput value="" onChange={noop} onClear={noop} placeholder="Pesquisar serviços" />,
    );
    expect(screen.getByLabelText("Pesquisar serviços")).toBeInTheDocument();
  });
});
