import { Tooltip, Spinner } from "@chakra-ui/react";

type ConnectionStatusProps = {
  connection: "connected" | "disconnected" | "desynchronized";
};

function ConnectionStatus({ connection }: ConnectionStatusProps) {
  return (
    <Tooltip label={
      {
        connected: "Connected",
        disconnected: "Connecting...",
        desynchronized: "Disconnected, please refresh",
      }[connection]
    }>
      <Spinner
        marginRight={2}
        size="xs"
        color={
          {
            connected: "green.500",
            disconnected: "orange.500",
            desynchronized: "red.500",
          }[connection]
        }
        bgColor={
          {
            connected: "green.500",
            disconnected: "",
            desynchronized: "",
          }[connection]
        }
        emptyColor={connection == "disconnected" ? 'transparent' : ''}
        speed={connection == "disconnected" ? '0.5s' : '0s'}
      />
    </Tooltip>
  );
}

export default ConnectionStatus;
