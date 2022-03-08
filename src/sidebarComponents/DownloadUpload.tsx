import { ButtonGroup } from "@chakra-ui/button";
import { Text } from "@chakra-ui/layout";
import "react";
import { VscCloudDownload, VscCloudUpload } from "react-icons/vsc";
import { getFileUploadWithDialog } from "../downloadUploadWrappers";
import { DecoratedButton } from "./DecoratedButton";

type DownloadUploadProps = {
  downloadFile: () => unknown;
  uploadFile: (file: File) => unknown;
  darkMode: boolean;
};

export function DownloadUpload({
  downloadFile,
  uploadFile,
  darkMode,
}: DownloadUploadProps) {
  return (
    <>
      <Text>You can also upload with drag and drop and download with Ctrl + S</Text>
      <ButtonGroup size="sm" display="flex">
        <DecoratedButton
          onClick={async (event) => {
            event.preventDefault();
            const file = await getFileUploadWithDialog();
            if (!file) return;
            uploadFile(file);
          }}
          icon={<VscCloudUpload />}
          darkMode={darkMode}
        >
          Upload
        </DecoratedButton>
        <DecoratedButton
          onClick={(event) => {
            event.preventDefault();
            downloadFile();
          }}
          icon={<VscCloudDownload />}
          darkMode={darkMode}
        >
          Download
        </DecoratedButton>
      </ButtonGroup>
    </>
  );
}
