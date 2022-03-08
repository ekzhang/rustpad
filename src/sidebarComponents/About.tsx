import { Link, Text } from "@chakra-ui/layout";
import "react";
import { VscRepoPull } from "react-icons/vsc";
import { DecoratedButton } from "./DecoratedButton";

type AboutProps = {
  loadRustpadSourceSample: () => unknown;
  darkMode: boolean;
};

export function About({ loadRustpadSourceSample, darkMode }: AboutProps) {
  return (
    <>
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

      <DecoratedButton
        onClick={loadRustpadSourceSample}
        icon={<VscRepoPull />}
        darkMode={darkMode}
      >
        Read the code
      </DecoratedButton>
    </>
  );
}
