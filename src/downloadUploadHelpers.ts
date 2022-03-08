/**
 * This appears to still be the best way to download a file while suggesting a filename.
 *
 * According to [mdn](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a#attr-download)
 * the download attribute gets ignored on URIs that are not either `same-origin` or use the `blob` or `data` schemes.
 */
export function downloadUri(uri: string, filename: string) {
  const downloadAnchor = document.createElement("a");
  downloadAnchor.download = filename;
  downloadAnchor.href = uri;
  downloadAnchor.click();
}

export function downloadText(text: string, fileName: string) {
  const file = new File([text], fileName);
  const url = URL.createObjectURL(file);
  downloadUri(url, fileName);
  URL.revokeObjectURL(url);
}

export async function getFileUploadWithDialog() {
  const uploadInput = document.createElement("input");
  uploadInput.type = "file";
  uploadInput.click();
  const controller = new AbortController();
  // await the user-input (selecting the file)
  await new Promise((resolve) =>
    uploadInput.addEventListener("change", resolve, {
      signal: controller.signal,
    })
  );
  controller.abort();
  const files = uploadInput.files;
  if (files?.length !== 1) return;
  return files[0];
}
