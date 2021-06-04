import { HStack, Icon, Text } from "@chakra-ui/react";
import { VscCircleFilled } from "react-icons/vsc";

type ConnectionStatusProps = {
  connection: "connected" | "disconnected" | "desynchronized";
};

function ConnectionStatus({ connection }: ConnectionStatusProps) {
  return (
    <HStack spacing={1}>
      <Icon
        as={VscCircleFilled}
        color={
          {
            connected: "green.500",
            disconnected: "orange.500",
            desynchronized: "red.500",
          }[connection]
        }
      />
      <Text fontSize="sm" fontStyle="italic" color="gray.600">
        {
          {
            connected: "You are connected!",
            disconnected: "Connecting to the server...",
            desynchronized: "Disconnected, please refresh.",
          }[connection]
        }
      </Text>
    </HStack>
  );
}

export default ConnectionStatus;
