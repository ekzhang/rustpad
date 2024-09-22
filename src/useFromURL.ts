import { useEffect, useState } from "react";
import useStorage from "use-local-storage-state";

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

function getParam<T>(
  key: string,
  type: "string" | "boolean",
  def: T = null,
): T | null
{
  let searchString = window.location.hash.split("?")[1];
  let searchParams = new URLSearchParams(typeof(searchString) === "string" ? searchString : "");
  if (searchParams.has(key)) {
    let value = searchParams.get(key);
    if (type === "boolean") {
      if (value === "true") {
        return true;
      } else if (value === "false") {
        return false;
      } else {
        return def;
      }
    }
    return value;
  } else {
    return def;
  }
}

function useParamOrElse<T, V>(
  key: string,
  type: "string" | "boolean",
  makeState: () => [V, (V) => null],
  transform: (T) => V | null,
): [V, (V) => null]
{
  let value = getParam(key, type);
  if (value !== null) {
    let transformed = transform(value);
    if (transformed !== null) {
      return useState(transformed);
    }
  }
  return makeState();
}

function useParamOrState<T, V>(
  key: string,
  type: "string" | "boolean",
  generator: () => V,
  transform: (T) => V | null = (val) => val,
): [V, (V) => null]
{
  return useParamOrElse(key, type, () => useState(generator), transform);
}

function useParamOrStorage<T, V>(
  key: string,
  type: "string" | "boolean",
  generator: () => V,
  transform: (T) => V | null = (val) => val,
): [V, (V) => null]
{
  return useParamOrElse(key, type, () => useStorage(key, generator), transform);
}

export { useHash, useParamOrState, useParamOrStorage };
