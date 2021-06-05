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
};

function User({ info, isMe = false, onChangeName, onChangeColor }: UserProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const { isOpen, onOpen, onClose } = useDisclosure();

  const nameColor = `hsl(${info.hue}, 90%, 25%)`;
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
          _hover={{ bgColor: "gray.200", cursor: "pointer" }}
          onClick={() => isMe && onOpen()}
        >
          <Icon as={VscAccount} />
          <Text fontWeight="medium" color={nameColor}>
            {info.name}
          </Text>
          {isMe && <Text>(you)</Text>}
        </HStack>
      </PopoverTrigger>
      <PopoverContent>
        <PopoverHeader fontWeight="semibold">Update Info</PopoverHeader>
        <PopoverArrow />
        <PopoverCloseButton />
        <PopoverBody>
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
            onClick={onChangeColor}
          >
            Change Color
          </Button>
        </PopoverBody>
        <PopoverFooter d="flex" justifyContent="flex-end">
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
