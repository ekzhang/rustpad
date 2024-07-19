import { useEffect, useState } from "react";

const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const idLen = 6;

function getHash() {
  if (!window.location.hash) {
    let newUrl = new URL(window.location);

    // Attempt retriving document ID as last part of path, moving it to the fragment
    let filename = newUrl.pathname.split("/").pop();
    if (filename != "" && filename != "index.html") {
      newUrl.pathname = newUrl.pathname.slice(0, newUrl.pathname - filename.length);
      newUrl.hash = filename;
    } else {
      // Generate random document ID as fallback / direct-hit
      let id = "";
      for (let i = 0; i < idLen; i++) {
        id += chars[Math.floor(Math.random() * chars.length)];
      }
      newUrl.hash = id;
    }
    window.history.replaceState(null, "", newUrl.href);
  }
  return window.location.hash.slice(1);
}

function useHash() {
  const [hash, setHash] = useState(getHash);

  useEffect(() => {
    const handler = () => setHash(getHash());
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  }, []);

  return hash;
}

export default useHash;
