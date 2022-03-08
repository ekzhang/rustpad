import { Button, ButtonOptions } from "@chakra-ui/button";

export function DecoratedButton({
  onClick,
  darkMode,
  icon,
  children,
}: React.PropsWithChildren<{
  onClick: React.MouseEventHandler<HTMLButtonElement>;
  icon: ButtonOptions["leftIcon"];
  darkMode?: boolean;
}>) {
  return (
    <Button
      size="sm"
      colorScheme={darkMode ? "whiteAlpha" : "blackAlpha"}
      borderColor={darkMode ? "purple.400" : "purple.600"}
      color={darkMode ? "purple.400" : "purple.600"}
      variant="outline"
      leftIcon={icon}
      mt={1}
      flex="auto"
      onClick={onClick}
      width="100%"
      minWidth="100%"
    >
      {children}
    </Button>
  );
}
