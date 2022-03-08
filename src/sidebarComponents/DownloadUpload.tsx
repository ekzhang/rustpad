import "react";
import { VscCloudDownload, VscCloudUpload } from "react-icons/vsc";
import { getFileUploadWithDialog } from "../downloadUploadHelpers";
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
  return (<>
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
        Upload [drag-and-drop]
      </DecoratedButton>
      <DecoratedButton
        onClick={(event) => {
          event.preventDefault();
          downloadFile();
        }}
        icon={<VscCloudDownload />}
        darkMode={darkMode}
      >
        Download [Meta + S]
      </DecoratedButton>
    </>
  );
}
