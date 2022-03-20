import {
  Editable,
  EditablePreview,
  EditableInput,
  HStack,
  Icon,
  Spacer,
  Stack,
  Text,
  useEditableControls,
  IconButton,
  Tooltip,
  Box,
} from "@chakra-ui/react";
import { FaPalette } from "react-icons/fa";
import { VscAccount, VscClose, VscEdit } from "react-icons/vsc";
import { UserInfo } from "./rustpad";
import React, { useContext } from "react";
import { ThemeContext } from "./Theme";

function makeColor(hue: number): string {
  const darkMode = useContext(ThemeContext).darkMode
  return `hsl(${hue}, 90%, ${darkMode ? "70%" : "25%"})`;
}

function User({ name, hue }: UserInfo) {

  return (
    <HStack>
      <Icon as={VscAccount}></Icon>
      <Text fontWeight="medium" color={makeColor(hue)}>
        {name}
      </Text>
    </HStack>
  );
}

function EditableControls() {
  const {
    isEditing,
    getEditButtonProps,
    getCancelButtonProps,
  } = useEditableControls();

  const darkMode = useContext(ThemeContext).darkMode;

  return isEditing ? (
    <IconButton aria-label="cancel" colorScheme={darkMode ? "white" : "gray"} size="xs" icon={<VscClose />} {...getCancelButtonProps()} />
  ) : (
    <IconButton aria-label="edit" colorScheme={darkMode ? "white" : "gray"} size="xs" icon={<VscEdit />} {...getEditButtonProps()} />
  )
}

type UserEditProps = {
  me: UserInfo;
  onChangeName: (name: string) => unknown;
  onChangeColor?: () => unknown;
}

function UserEdit({
  me,
  onChangeName,
  onChangeColor,
}: UserEditProps) {
  const theme = useContext(ThemeContext);
  const colorScheme = theme.darkMode ? "white" : "gray"

  return (
    <Editable placeholder={me.name} defaultValue={me.name} submitOnBlur={true} onSubmit={onChangeName}>
      <HStack>
        <Tooltip label="Change your color">
          <IconButton
            colorScheme={colorScheme}
            size="xxs"
            aria-label="change color"
            color={makeColor(me.hue)}
            icon={<FaPalette />}
            onClick={onChangeColor}></IconButton>
        </Tooltip>
        <EditableInput fontWeight="medium" color={makeColor(me.hue)} textAlign="left" maxLength={32} />
        <Tooltip label="Edit your display name">
          <EditablePreview fontWeight="medium" color={makeColor(me.hue)} textAlign="left" />
        </Tooltip>
        <YouLabel />
        <Spacer />
        <EditableControls />
      </HStack >
    </Editable>
  );
}
type UserListProps = {
  users: Record<number, UserInfo>;
} & UserEditProps;

function YouLabel() {
  const { isEditing, getEditButtonProps } = useEditableControls()
  return <Box {...getEditButtonProps()} textAlign="left"> {isEditing ? "" : "(you)"}</Box>
}

function UserList({ users, me, onChangeName, onChangeColor }: UserListProps) {
  return (
    <Stack spacing={0} mb={1.5} fontSize="sm">
      {Object.entries(users).map(([id, info]) => (
        <User key={id} {...info} />
      ))}
      <UserEdit
        me={me}
        onChangeName={onChangeName}
        onChangeColor={onChangeColor}
      />
    </Stack>
  )
}

export default UserList;