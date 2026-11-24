import { Box, CloseButton, Flex, Spinner, Text } from "@chakra-ui/react";

import type { RunResult, RunStage, UserInfo } from "./rustpad";

/** What the output panel shows. */
export type RunState = {
  status: "idle" | "running" | "finished" | "failed";
  by: UserInfo | null;
  result?: RunResult;
  error?: string;
};

/** Monaco's default font stack, so output matches the editor. */
const monospace = 'Menlo, Monaco, "Courier New", monospace';

function hasOutput(stage: RunStage): boolean {
  return stage.code !== 0 || stage.stdout !== "" || stage.stderr !== "";
}

function summary(result: RunResult): string {
  const stage = result.run ?? result.compile;
  const outcome = stage?.status ?? stage?.signal ?? `exit ${stage?.code ?? "?"}`;
  const compileFailed = result.run === null ? "compile failed · " : "";
  return `${result.runtime} ${result.version} · ${result.duration_ms} ms · ${compileFailed}${outcome}`;
}

function Stream(props: { text: string; error?: boolean; darkMode: boolean }) {
  if (props.text === "") return null;
  const errorColor = props.darkMode ? "#f48771" : "#a1260d";
  return (
    <Box
      as="pre"
      m={0}
      fontFamily="inherit"
      whiteSpace="pre-wrap"
      wordBreak="break-all"
      color={props.error ? errorColor : undefined}
    >
      {props.text}
    </Box>
  );
}

function StageOutput(props: { stage: RunStage; darkMode: boolean }) {
  return (
    <>
      <Stream text={props.stage.stdout} darkMode={props.darkMode} />
      <Stream text={props.stage.stderr} error darkMode={props.darkMode} />
      {props.stage.truncated && (
        <Text color="gray.500">[output truncated]</Text>
      )}
    </>
  );
}

function RunOutput(props: {
  run: RunState;
  darkMode: boolean;
  onClose: () => void;
}) {
  const { run, darkMode } = props;
  const name = run.by?.name ?? "someone";
  const compile = run.result?.compile;

  let header = "";
  if (run.status === "running") header = `Running · started by ${name}`;
  if (run.status === "finished" && run.result)
    header = `Run by ${name} · ${summary(run.result)}`;
  if (run.status === "failed") header = `Run by ${name} · failed`;

  return (
    <Flex
      direction="column"
      h="30%"
      flexShrink={0}
      borderTopWidth="1px"
      borderColor={darkMode ? "#3c3c3c" : "#e8e8e8"}
      bgColor={darkMode ? "#252526" : "#f3f3f3"}
    >
      <Flex
        align="center"
        h={7}
        px={3.5}
        flexShrink={0}
        color="#888888"
        fontSize="13px"
        fontWeight="medium"
      >
        {run.status === "running" && <Spinner size="xs" mr={2} />}
        <Text flex={1} noOfLines={1}>
          {header}
        </Text>
        <CloseButton size="sm" onClick={props.onClose} />
      </Flex>
      <Box
        flex={1}
        minH={0}
        overflowY="auto"
        px={3.5}
        pb={2}
        fontFamily={monospace}
        fontSize="13px"
      >
        {run.status === "failed" && (
          <Stream text={run.error ?? ""} error darkMode={darkMode} />
        )}
        {compile && hasOutput(compile) && (
          <StageOutput stage={compile} darkMode={darkMode} />
        )}
        {run.result?.run && (
          <StageOutput stage={run.result.run} darkMode={darkMode} />
        )}
      </Box>
    </Flex>
  );
}

export default RunOutput;
