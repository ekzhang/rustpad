import { useEffect, useState } from "react";

const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const idLen = 6;

function isValid(hash: string): boolean {
  return hash.length === idLen && /^[A-Za-z0-9]+$/.test(hash);
}

function getHash() {
  const hash = window.location.hash;

  if (!hash || !isValid(hash)) {
    let id = "";
    for (let i = 0; i < idLen; i++) {
      id += chars[Math.floor(Math.random() * chars.length)];
    }
    window.history.replaceState(null, "", "#" + id);
    return id;
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
