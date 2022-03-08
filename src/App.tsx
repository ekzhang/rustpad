import React, { useEffect, useRef, useState } from "react";
import {
  Box,
  Container,
  Flex,
  Heading,
  HStack,
  Icon,
  Switch,
  Text,
} from "@chakra-ui/react";
import { VscChevronRight, VscFolderOpened, VscGist } from "react-icons/vsc";
import useStorage from "use-local-storage-state";
import Editor from "@monaco-editor/react";
import { editor } from "monaco-editor/esm/vs/editor/editor.api";
import rustpadRaw from "../rustpad-server/src/rustpad.rs?raw";
import { getFileExtension, getLanguage, Language } from "./languages";
import animals from "./animals.json";
import Rustpad, { UserInfo } from "./rustpad";
import useHash from "./useHash";
import ConnectionStatus, { ConnectionStatusState } from "./ConnectionStatus";
import Footer from "./Footer";
import { generateHue, DisplayUsers } from "./sidebarComponents/DisplayUsers";
import { ShareLink } from "./sidebarComponents/ShareLink";
import { LanguageSelection } from "./sidebarComponents/LanguageSelection";
import { About } from "./sidebarComponents/About";
import { DownloadUpload } from "./sidebarComponents/DownloadUpload";
import { useCustomToasts } from "./useCustomToasts";
import { useKeyboardCtrlIntercept } from "./useKeyboardCtrlIntercept";
import { downloadText } from "./downloadUploadWrappers";

function getWsUri(id: string) {
  return (
    (window.location.origin.startsWith("https") ? "wss://" : "ws://") +
    window.location.host +
    `/api/socket/${id}`
  );
}

function generateName() {
  return "Anonymous " + animals[Math.floor(Math.random() * animals.length)];
}

function App() {
  const toasts = useCustomToasts();
  const [language, setLanguage] = useState<Language>("plaintext");
  const [connection, setConnection] =
    useState<ConnectionStatusState>("disconnected");
  const [users, setUsers] = useState<Record<number, UserInfo>>({});
  const [name, setName] = useStorage("name", generateName);
  const [hue, setHue] = useStorage("hue", generateHue);
  const [editor, setEditor] = useState<editor.IStandaloneCodeEditor>();
  const [darkMode, setDarkMode] = useStorage("darkMode", () => false);
  const rustpad = useRef<Rustpad>();
  const id = useHash();

  useEffect(() => {
    if (editor?.getModel()) {
      const model = editor.getModel()!;
      model.setValue("");
      model.setEOL(0); // LF
      rustpad.current = new Rustpad({
        uri: getWsUri(id),
        editor,
        onConnected: () => setConnection("connected"),
        onDisconnected: () => setConnection("disconnected"),
        onDesynchronized: () => {
          setConnection("desynchronized");
          toasts.desynchronized();
        },
        onChangeLanguage: setLanguage,
        onChangeUsers: setUsers,
      });
      return () => {
        rustpad.current?.dispose();
        rustpad.current = undefined;
      };
    }
  }, [id, editor, toasts, setUsers]);

  useEffect(() => {
    if (connection === "connected") {
      rustpad.current?.setInfo({ name, hue });
    }
  }, [connection, name, hue]);

  function setText(newText: string) {
    const model = editor?.getModel();
    if (!model || !editor) return;
    model.pushEditOperations(
      editor.getSelections(),
      [
        {
          range: model.getFullModelRange(),
          text: newText,
        },
      ],
      () => null
    );
    editor.setPosition({ column: 0, lineNumber: 0 });
  }

  async function uploadFile(file: File) {
    const text = await file.text();
    setText(text);
    const newLanguage = getLanguage(file.name);
    setLanguage(newLanguage);
    toasts.fileUpload(name);
  }

  function downloadFile() {
    const model = editor?.getModel();
    if (!model) return;
    downloadText(model.getValue(), `rustpad.${getFileExtension(language)}`);
  }

  useKeyboardCtrlIntercept("s", downloadFile);

  function toggleDarkMode() {
    setDarkMode((darkMode) => !darkMode);
  }

  function loadRustpadSourceSample() {
    setText(rustpadRaw);
    setLanguage("rust");
  }

  return (
    <Flex
      direction="column"
      h="100vh"
      overflow="hidden"
      bgColor={darkMode ? "#1e1e1e" : "white"}
      color={darkMode ? "#cbcaca" : "inherit"}
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => {
        event.preventDefault();
        const dragItems = event.dataTransfer.items;
        if (dragItems.length !== 1) return;
        const file = dragItems[0].getAsFile();
        if (file === null) return;
        uploadFile(file);
      }}
    >
      <Box
        flexShrink={0}
        bgColor={darkMode ? "#333333" : "#e8e8e8"}
        color={darkMode ? "#cccccc" : "#383838"}
        textAlign="center"
        fontSize="sm"
        py={0.5}
      >
        Rustpad
      </Box>
      <Flex flex="1 0" minH={0}>
        <Container
          w="xs"
          bgColor={darkMode ? "#252526" : "#f3f3f3"}
          overflowY="auto"
          maxW="full"
          lineHeight={1.4}
          py={4}
        >
          <ConnectionStatus darkMode={darkMode} connection={connection} />

          <Flex justifyContent="space-between" mt={4} mb={1.5} w="full">
            <Heading size="sm">Dark Mode</Heading>
            <Switch isChecked={darkMode} onChange={toggleDarkMode} />
          </Flex>

          <SideBarGroup title="Language">
            <LanguageSelection
              {...{
                language,
                setLanguage,
                darkMode,
              }}
            />
          </SideBarGroup>

          <SideBarGroup title="Share Link">
            <ShareLink
              {...{
                id,
                darkMode,
              }}
            />
          </SideBarGroup>

          <SideBarGroup title="Upload and Download">
            <DownloadUpload
              {...{
                uploadFile,
                downloadFile,
                darkMode,
              }}
            />
          </SideBarGroup>

          <SideBarGroup title="Active Users">
            <DisplayUsers
              {...{
                users,
                name,
                setName,
                hue,
                setHue,
                darkMode,
              }}
            />
          </SideBarGroup>

          <SideBarGroup title="About">
            <About
              {...{
                loadRustpadSourceSample,
                darkMode,
              }}
            />
          </SideBarGroup>
        </Container>
        <Flex flex={1} minW={0} h="100%" direction="column" overflow="hidden">
          <HStack
            h={6}
            spacing={1}
            color="#888888"
            fontWeight="medium"
            fontSize="13px"
            px={3.5}
            flexShrink={0}
          >
            <Icon as={VscFolderOpened} fontSize="md" color="blue.500" />
            <Text>documents</Text>
            <Icon as={VscChevronRight} fontSize="md" />
            <Icon as={VscGist} fontSize="md" color="purple.500" />
            <Text>{id}</Text>
          </HStack>
          <Box flex={1} minH={0}>
            <Editor
              theme={darkMode ? "vs-dark" : "vs"}
              language={language}
              options={{
                automaticLayout: true,
                fontSize: 13,
              }}
              onMount={(editor) => setEditor(editor)}
            />
          </Box>
        </Flex>
      </Flex>
      <Footer />
    </Flex>
  );
}

function SideBarGroup({
  title,
  children,
}: React.PropsWithChildren<{ title: string }>) {
  return (
    <>
      <Heading mt={4} mb={1.5} size="sm">
        {title}
      </Heading>
      {children}
    </>
  );
}

export default App;
