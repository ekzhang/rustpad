import { useEffect, useState } from "react";

const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const idLen = 6;

function getHash() {
  let newUrl = new URL(window.location);
  if (!window.location.hash) {
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
  }

  // Move parameters after document ID if present
  if (newUrl.search) {
    if (newUrl.hash.includes("?")) {
      newUrl.hash += "&" + newUrl.search.slice(1);
    } else {
      newUrl.hash += newUrl.search;
    }
    newUrl.search = "";
  }

  if (newUrl.href != window.location.href) {
    window.history.replaceState(null, "", newUrl.href);
  }

  return newUrl.hash.slice(1).split("?")[0];
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
