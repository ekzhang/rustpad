import { Flex, Icon, Text } from "@chakra-ui/react";
import { VscRemote } from "react-icons/vsc";

const version =
  typeof import.meta.env.VITE_SHA === "string"
    ? import.meta.env.VITE_SHA.slice(0, 7)
    : "development";

function Footer() {
  return (
    <Flex h="22px" bgColor="#0071c3" color="white">
      <Flex
        h="100%"
        bgColor="#09835c"
        pl={2.5}
        pr={4}
        fontSize="sm"
        align="center"
      >
        <Icon as={VscRemote} mb={-0.5} mr={1} />
        <Text fontSize="xs">Rustpad ({version})</Text>
      </Flex>
    </Flex>
  );
}

export default Footer;
