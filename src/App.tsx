import { useEffect } from "react";
import { set_panic_hook } from "rustpad-wasm";
import {
  Box,
  Container,
  Flex,
  Heading,
  HStack,
  Icon,
  Input,
  Link,
  Select,
  Stack,
  Text,
} from "@chakra-ui/react";
import { VscAccount, VscRemote } from "react-icons/vsc";
import Editor from "@monaco-editor/react";
import Rustpad from "./rustpad";

set_panic_hook();

const WS_URI =
  (window.location.origin.startsWith("https") ? "wss://" : "ws://") +
  window.location.host +
  "/api/socket";

function App() {
  useEffect(() => {
    const rustpad = new Rustpad({
      uri: WS_URI,
      onConnected: () => console.log("connected!"),
      onDisconnected: () => console.log("disconnected!"),
    });
    return () => rustpad.dispose();
  }, []);

  return (
    <Flex direction="column" h="100vh" overflow="hidden">
      <Box
        flexShrink={0}
        bgColor="#e8e8e8"
        color="#383838"
        textAlign="center"
        fontSize="sm"
        py={0.5}
      >
        Rustpad
      </Box>
      <Flex flex="1 0" minH={0}>
        <Flex direction="column" bgColor="#f3f3f3" w="sm" overflowY="auto">
          <Container maxW="full" lineHeight={1.4}>
            <Heading mt={4} mb={1.5} size="sm">
              Share Link
            </Heading>
            <Input
              readOnly
              size="sm"
              variant="outline"
              value={`${window.location.origin}/`}
            />

            <Heading mt={4} mb={1.5} size="sm">
              Language
            </Heading>
            <Select size="sm">
              <option value="text">Text</option>
            </Select>

            <Heading mt={4} mb={1.5} size="sm">
              Active Users
            </Heading>
            <Stack mb={1.5} fontSize="sm">
              <HStack p={2} rounded="md" _hover={{ bgColor: "gray.200" }}>
                <Icon as={VscAccount} />
                <Text fontWeight="medium">Anonymous Bear</Text>
                <Text>(you)</Text>
              </HStack>
            </Stack>

            <Heading mt={4} mb={1.5} size="sm">
              About
            </Heading>
            <Text fontSize="sm" mb={1.5}>
              <strong>Rustpad</strong> is an open-source collaborative text
              editor based on the <em>operational transformation</em> algorithm.
            </Text>
            <Text fontSize="sm" mb={1.5}>
              Share a link to this pad with others, and they can edit it from
              their browser while seeing your changes in real time.
            </Text>
            <Text fontSize="sm" mb={1.5}>
              Built using Rust and TypeScript. See the{" "}
              <Link
                color="blue.600"
                href="https://github.com/ekzhang/rustpad"
                isExternal
              >
                GitHub repository
              </Link>{" "}
              for details.
            </Text>
          </Container>
        </Flex>
        <Editor
          theme="vs"
          options={{
            automaticLayout: true,
            fontSize: 14,
            quickSuggestions: false,
            parameterHints: {
              enabled: false,
            },
            suggestOnTriggerCharacters: false,
            acceptSuggestionOnEnter: "off",
            tabCompletion: "off",
            wordBasedSuggestions: false,
          }}
        />
      </Flex>
      <Flex h="22px" bgColor="#0071c3">
        <Flex
          h="100%"
          bgColor="#09835c"
          pl={2.5}
          pr={4}
          color="#eeeeef"
          fontSize="sm"
          align="center"
        >
          <Icon as={VscRemote} mb={-0.5} mr={1} />
          <Text fontSize="xs">Rustpad v0.1.0</Text>
        </Flex>
      </Flex>
    </Flex>
  );
}

export default App;
