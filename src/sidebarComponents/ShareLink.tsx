import { Button } from "@chakra-ui/button";
import { Input, InputGroup, InputRightElement } from "@chakra-ui/input";
import "react";
import { useCustomToasts } from "../useCustomToasts";

type ShareLinkProps = { darkMode: boolean; id: string };

export function ShareLink({ darkMode, id }: ShareLinkProps) {
  const toasts = useCustomToasts();

  async function handleCopy() {
    await navigator.clipboard.writeText(`${window.location.origin}/#${id}`);
    toasts.copyToClipBoard();
  }

  return (
    <InputGroup size="sm">
      <Input
        readOnly
        pr="3.5rem"
        variant="outline"
        bgColor={darkMode ? "#3c3c3c" : "white"}
        borderColor={darkMode ? "#3c3c3c" : "white"}
        value={`${window.location.origin}/#${id}`}
      />
      <InputRightElement width="3.5rem">
        <Button
          h="1.4rem"
          size="xs"
          onClick={handleCopy}
          _hover={{ bg: darkMode ? "#575759" : "gray.200" }}
          bgColor={darkMode ? "#575759" : "gray.200"}
        >
          Copy
        </Button>
      </InputRightElement>
    </InputGroup>
  );
}
