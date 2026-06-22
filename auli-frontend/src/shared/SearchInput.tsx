import type { ChangeEvent, Ref } from "react";
import { Box, Input, chakra } from "@chakra-ui/react";
import { MdSearch, MdClose } from "react-icons/md";

interface SearchInputProps {
  value: string;
  onChange: (e: ChangeEvent<HTMLInputElement>) => void;
  onClear: () => void;
  placeholder?: string;
  "aria-label"?: string;
  // React 19: ref is a normal prop on function components, no forwardRef needed.
  ref?: Ref<HTMLInputElement>;
}

/**
 * Sticky-bar search field shared by the Serviços, FAQs, and Conteúdos lists:
 * leading search icon, the input, and a trailing clear button shown only when
 * there's a query. The clear control is a real <button> so it's keyboard- and
 * screen-reader-accessible.
 */
export function SearchInput({
  value,
  onChange,
  onClear,
  placeholder,
  "aria-label": ariaLabel,
  ref,
}: SearchInputProps) {
  return (
    <Box position="relative">
      <Box
        position="absolute"
        left={3}
        top="50%"
        style={{ transform: "translateY(-50%)" }}
        color="fg.muted"
        display="flex"
        zIndex={1}
      >
        <MdSearch size={17} />
      </Box>
      <Input
        ref={ref}
        placeholder={placeholder}
        aria-label={ariaLabel ?? placeholder}
        value={value}
        onChange={onChange}
        bg="bg.canvas"
        size="md"
        h="36px"
        borderRadius="md"
        paddingLeft="36px"
        paddingRight={value ? "36px" : "12px"}
      />
      {value && (
        <chakra.button
          type="button"
          aria-label="Limpar busca"
          position="absolute"
          right={3}
          top="50%"
          style={{ transform: "translateY(-50%)", cursor: "pointer" }}
          color="fg.muted"
          display="flex"
          zIndex={1}
          onClick={onClear}
          _hover={{ color: "fg" }}
        >
          <MdClose size={16} />
        </chakra.button>
      )}
    </Box>
  );
}
