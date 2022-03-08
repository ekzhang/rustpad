import { Language } from "./languages";
import "react";
import { useToast, Text } from "@chakra-ui/react";
import { useMemo } from "react";

export function useCustomToasts() {
  const toast = useToast();
  const messages = useMemo(
    () => ({
      copyToClipBoard: () =>
        toast({
          title: "Copied!",
          description: "Link copied to clipboard",
          status: "success",
          duration: 2000,
          isClosable: true,
        }),
      languageChange: (newLanguage: Language) =>
        toast({
          title: "Language updated",
          description: (
            <>
              All users are now editing in{" "}
              <Text as="span" fontWeight="semibold">
                {newLanguage}
              </Text>
              .
            </>
          ),
          status: "info",
          duration: 2000,
          isClosable: true,
        }),
      desynchronized: () =>
        toast({
          title: "Desynchronized with server",
          description: "Please save your work and refresh the page.",
          status: "error",
          duration: null,
        }),
      fileUpload: (user: string) =>
        toast({
          title: "File uploaded",
          description: `A file has been uploaded by ${user}, the edited text has been changed and the language updated accordingly`,
          status: "info",
          duration: 5000,
          isClosable: true,
        }),
    }),
    [toast]
  );
  return messages;
}
