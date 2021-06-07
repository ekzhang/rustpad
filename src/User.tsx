import {
  Button,
  ButtonGroup,
  HStack,
  Icon,
  Input,
  Popover,
  PopoverArrow,
  PopoverBody,
  PopoverCloseButton,
  PopoverContent,
  PopoverFooter,
  PopoverHeader,
  PopoverTrigger,
  Text,
  useDisclosure,
} from "@chakra-ui/react";
import { useRef } from "react";
import { FaPalette } from "react-icons/fa";
import { VscAccount } from "react-icons/vsc";
import { UserInfo } from "./rustpad";

type UserProps = {
  info: UserInfo;
  isMe?: boolean;
  onChangeName?: (name: string) => unknown;
  onChangeColor?: () => unknown;
  darkMode: boolean;
};

function User({
  info,
  isMe = false,
  onChangeName,
  onChangeColor,
  darkMode,
}: UserProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const { isOpen, onOpen, onClose } = useDisclosure();

  const nameColor = `hsl(${info.hue}, 90%, ${darkMode ? "70%" : "25%"})`;
  return (
    <Popover
      placement="right"
      isOpen={isOpen}
      onClose={onClose}
      initialFocusRef={inputRef}
    >
      <PopoverTrigger>
        <HStack
          p={2}
          rounded="md"
          _hover={{
            bgColor: darkMode ? "#464647" : "gray.200",
            cursor: "pointer",
          }}
          onClick={() => isMe && onOpen()}
        >
          <Icon color={darkMode ? "#cbcaca" : undefined} as={VscAccount} />
          <Text fontWeight="medium" color={nameColor}>
            {info.name}
          </Text>
          {isMe && <Text color={darkMode ? "#cbcaca" : undefined}>(you)</Text>}
        </HStack>
      </PopoverTrigger>
      <PopoverContent
        bg={darkMode ? "#333333" : "white"}
        color={darkMode ? "#cbcaca" : undefined}
        borderColor={darkMode ? "#464647" : "gray.200"}
      >
        <PopoverHeader
          fontWeight="semibold"
          borderColor={darkMode ? "#464647" : "gray.200"}
        >
          Update Info
        </PopoverHeader>
        <PopoverArrow bg={darkMode ? "#333333" : "white"} />
        <PopoverCloseButton />
        <PopoverBody borderColor={darkMode ? "#464647" : "gray.200"}>
          <Input
            ref={inputRef}
            mb={2}
            value={info.name}
            maxLength={25}
            onChange={(event) => onChangeName?.(event.target.value)}
          />
          <Button
            size="sm"
            w="100%"
            leftIcon={<FaPalette />}
						_hover={{bg: darkMode ? "#464647" : "gray.200"}}
            bg={darkMode ? "#464647" : "gray.200"}
            onClick={onChangeColor}
          >
            Change Color
          </Button>
        </PopoverBody>
        <PopoverFooter
          d="flex"
          justifyContent="flex-end"
          borderColor={darkMode ? "#464647" : "gray.200"}
        >
          <ButtonGroup size="sm">
            <Button colorScheme="blue" onClick={onClose}>
              Done
            </Button>
          </ButtonGroup>
        </PopoverFooter>
      </PopoverContent>
    </Popover>
  );
}

export default User;
