import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogOverlay,
  Box,
  Button, Container, Drawer, DrawerBody,
  DrawerCloseButton, DrawerContent, DrawerHeader, DrawerOverlay, Flex,
  Heading,
  HStack,
  Icon, IconButton, Input, InputGroup, InputLeftElement, InputRightElement, Link,
  Popover, PopoverCloseButton,
  PopoverContent, PopoverTrigger,
  Select, Spacer, Text, Tooltip, useDisclosure, useToast
} from "@chakra-ui/react";
import Editor from "@monaco-editor/react";
import { editor } from "monaco-editor/esm/vs/editor/editor.api";
import { useContext, useEffect, useRef, useState } from "react";
import { HiUserGroup, HiUser } from "react-icons/hi";
import { FaMoon, FaSun } from "react-icons/fa";
import {
  VscChevronRight,
  VscFolderOpened,
  VscGist,
  VscLink, VscRepoPull
} from "react-icons/vsc";
import useStorage from "use-local-storage-state";
import rustpadRaw from "../rustpad-server/src/rustpad.rs?raw";
import animals from "./animals.json";
import ConnectionStatus from "./ConnectionStatus";
import Footer from "./Footer";
import languages from "./languages.json";
import Rustpad, { UserInfo } from "./rustpad";
import useHash from "./useHash";
import UserList from "./User";
import React, { Component } from 'react';
import { ThemeContext, DarkModeToggle } from "./Theme";


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

function generateHue() {
  return Math.floor(Math.random() * 360);
}

function App() {
  const toast = useToast();
  const [language, setLanguage] = useState("plaintext");
  const [connection, setConnection] = useState<
    "connected" | "disconnected" | "desynchronized"
  >("disconnected");
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
          toast({
            title: "Desynchronized with server",
            description: "Please save your work and refresh the page.",
            status: "error",
            duration: null,
          });
        },
        onChangeLanguage: (language) => {
          if (languages.includes(language)) {
            setLanguage(language);
          }
        },
        onChangeUsers: setUsers,
      });
      return () => {
        rustpad.current?.dispose();
        rustpad.current = undefined;
      };
    }
  }, [id, editor, toast, setUsers]);

  useEffect(() => {
    if (connection === "connected") {
      rustpad.current?.setInfo({ name, hue });
    }
  }, [connection, name, hue]);

  function handleChangeLanguage(language: string) {
    setLanguage(language);
    if (rustpad.current?.setLanguage(language)) {
      toast({
        title: "Language updated",
        description: (
          <>
            All users are now editing in{" "}
            <Text as="span" fontWeight="semibold">
              {language}
            </Text>
            .
          </>
        ),
        status: "info",
        duration: 2000,
        isClosable: true,
      });
    }
  }



  async function handleCopy() {
    await navigator.clipboard.writeText(`${window.location.origin}/#${id}`);
    toast({
      title: "Copied!",
      description: "Link copied to clipboard",
      status: "success",
      duration: 2000,
      isClosable: true,
    });
  }


  function toggleDarkMode() {
    setDarkMode(!darkMode);
  }


  const handleLoadSample = () => {
    if (editor?.getModel()) {
      const model = editor.getModel()!;
      model.pushEditOperations(
        editor.getSelections(),
        [
          {
            range: model.getFullModelRange(),
            text: rustpadRaw,
          },
        ],
        () => null
      );
      editor.setPosition({ column: 0, lineNumber: 0 });
      if (language !== "rust") {
        handleChangeLanguage("rust");
      }
    }
  }


  const aboutDrawer = useDisclosure()


  return (
    <ThemeContext.Provider value={{ darkMode, toggleDarkMode }}>
      <Flex
        direction="column"
        h="100vh"
        overflow="hidden"
        bgColor={darkMode ? "#1e1e1e" : "white"}
        color={darkMode ? "#cbcaca" : "inherit"}
      >

        <Box
          flexShrink={2}
          bgColor={darkMode ? "#333333" : "#e8e8e8"}
          color={darkMode ? "#cccccc" : "#383838"}
          textAlign="center"
          fontSize="sm"
        >
          <Flex spacing={0} px={2} alignItems="center">
            <ConnectionStatus connection={connection} />

            <Button
              h="1.4rem"
              fontWeight="normal"
              rounded="none"
              _hover={{ bg: darkMode ? "#575759" : "gray.200" }}
              bgColor={darkMode ? "#333333" : "gray.200"}
              px={2}
              onClick={aboutDrawer.onOpen}
            >
              Rustpad
            </Button>

            <Tooltip label="Syntax highlighting">
              <Select
                maxWidth="10em"
                size="xs"
                bgColor={darkMode ? "#3c3c3c" : "white"}
                _hover={{ bgColor: darkMode ? "#575757" : "dddddd" }}
                borderColor={darkMode ? "#3c3c3c" : "white"}
                value={language}
                onChange={(event) => handleChangeLanguage(event.target.value)}

              >
                {languages.map((lang) => (
                  <option key={lang} value={lang} style={{ color: "black" }}>
                    {lang}
                  </option>
                ))}
              </Select>
            </Tooltip>
            <Spacer />
            <Users
              users={users}
              me={{ name, hue }}
              setName={setName}
              setHue={setHue}
              id={id}
              handleCopy={handleCopy}
            />
            <DarkModeToggle />
          </Flex>
        </Box >
        <Flex flex="1 0" minH={0}>

          <Drawer
            isOpen={aboutDrawer.isOpen}
            onClose={aboutDrawer.onClose}
            placement="left"
          >
            <AboutDrawer loadSample={handleLoadSample} />
          </Drawer>
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
              <Text>rustpad</Text>
              <Icon as={VscChevronRight} fontSize="md" />
              <Link onClick={handleCopy} color="#bbbbbb" _hover={{ color: "#ffffff" }} >
                <HStack spacing={1} >
                  <Icon as={VscGist} fontSize="md" color="purple.500" />
                  <Text>{id}</Text>
                  <Icon as={VscLink} fontSize="md" color="grey.500" />
                </HStack>
              </Link>
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
      </Flex >
    </ThemeContext.Provider>
  );
}

export default App;

type UsersProps = {
  users: Record<number, UserInfo>;
  me: UserInfo;
  setName: (name: string) => unknown;
  setHue: (hue: number) => unknown;
  id: string;
  handleCopy: () => unknown;
};


function Users({ users, me, setName, setHue, id, handleCopy }: UsersProps) {
  const [usersIsOpen, setUsersIsOpen] = useState(false)
  const open = () => setUsersIsOpen(!usersIsOpen)
  const close = () => setUsersIsOpen(false)
  const userCount = () => Object.entries(users).length
  const darkMode = useContext(ThemeContext).darkMode;

  return (
    <Popover
      isOpen={usersIsOpen}
      onClose={close}
      placement='bottom'
      closeOnBlur={false}
    >
      <PopoverTrigger>
        <Button size="xs"
          onClick={open}
          rounded="none"
          bgColor={
            usersIsOpen ?
              (darkMode ? "#444444" : "#dddddd") :
              (darkMode ? "#333333" : "#e8e8e8")
          }
          _hover={{}} >
          <Icon as={userCount() > 0 ? HiUserGroup : HiUser} />
          <Text px={1}>{userCount() > 0 ?
            (userCount() + " other editor" + (userCount() > 1 ? "s" : "")) :
            "editing alone"}</Text>
        </Button>
      </PopoverTrigger>
      <PopoverContent
        borderColor={darkMode ? "#222222" : "#999999"}
        bgColor={darkMode ? "#333333" : "#e8e8e8"}
        color={darkMode ? "#cbcaca" : "inherit"}
        paddingBottom={5}
      >
        <PopoverCloseButton />
        <Container>
          <Heading mt={4} mb={1.5} size="sm">
            Active Users
          </Heading>
          <UserList
            users={users}
            me={me}
            onChangeName={(name) => name.length > 0 && setName(name)}
            onChangeColor={() => setHue(generateHue())}
          />

          <Heading mt={4} mb={1.5} size="sm">
            Share Link
          </Heading>
          <InputGroup size="sm">
            <InputLeftElement>
              <Icon as={VscLink} color="grey.500" />
            </InputLeftElement>
            <Input
              readOnly
              pr="3.5rem"
              variant="flushed"
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
        </Container>
      </PopoverContent>
    </Popover>
  )
}

type AboutBoxProps = {
  loadSample: () => unknown;
}


function AboutDrawer({ loadSample }: AboutBoxProps) {
  const darkMode = useContext(ThemeContext).darkMode;

  return (
    <>
      <DrawerOverlay />
      <DrawerContent
        bgColor={darkMode ? "#1e1e1e" : "white"}
        color={darkMode ? "#cbcaca" : "inherit"}
      >
        <DrawerCloseButton />
        <DrawerHeader>About <Link href="/">Rustpad</Link></DrawerHeader>

        <DrawerBody>

          <Text fontSize="sm" mb={1.5}>
            <strong>Rustpad</strong> is an open-source collaborative text editor
            based on the <em>operational transformation</em> algorithm.
          </Text>
          <Text fontSize="sm" mb={1.5}>
            Share a link to this pad with others, and they can edit from their
            browser while seeing your changes in real time.
          </Text>
          <Text fontSize="sm" mb={1.5}>
            Built using Rust and TypeScript. See the{" "}
            <Link
              color="blue.600"
              fontWeight="semibold"
              href="https://github.com/ekzhang/rustpad"
              isExternal
            >
              GitHub repository
            </Link>{" "}
            for details.
          </Text>

          <LoadSampleButton loadSample={loadSample} />

        </DrawerBody>
      </DrawerContent>
    </>
  )
}

function LoadSampleButton({ loadSample }: AboutBoxProps) {
  const { isOpen, onOpen, onClose } = useDisclosure()
  const cancelRef = useRef<HTMLButtonElement>(null)
  const darkMode = useContext(ThemeContext).darkMode;

  return (
    <>
      <Button
        size="sm"
        colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
        borderColor={darkMode ? "purple.400" : "purple.600"}
        color={darkMode ? "purple.400" : "purple.600"}
        variant="outline"
        leftIcon={<VscRepoPull />}
        mt={1}
        onClick={onOpen}
      >
        Read the code
      </Button>

      <AlertDialog
        isOpen={isOpen}
        leastDestructiveRef={cancelRef}
        onClose={onClose}
      >
        <AlertDialogOverlay>
          <AlertDialogContent

            bgColor={darkMode ? "#333333" : "#e8e8e8"}
            color={darkMode ? "#cccccc" : "#383838"}
          >
            <AlertDialogHeader fontSize='lg' fontWeight='bold'>
              Load Rustpad code
            </AlertDialogHeader>

            <AlertDialogBody>
              Are you sure? This will overwrite the current session.
            </AlertDialogBody>

            <AlertDialogFooter>
              <Button
                colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"} ref={cancelRef} onClick={onClose}>
                Cancel
              </Button>
              <Button colorScheme='red' onClick={() => { loadSample(); onClose() }} ml={3}>
                Yes
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </>
  )
}