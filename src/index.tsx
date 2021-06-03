import { StrictMode } from "react";
import ReactDOM from "react-dom";
import { ChakraProvider } from "@chakra-ui/react";
import "./index.css";

const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const idLen = 6;

if (!window.location.hash) {
  let id = "";
  for (let i = 0; i < idLen; i++) {
    id += chars[Math.floor(Math.random() * chars.length)];
  }
  window.location.hash = id;
}

// An asynchronous entry point is needed to load WebAssembly files.
import("./App").then(({ default: App }) => {
  ReactDOM.render(
    <StrictMode>
      <ChakraProvider>
        <App />
      </ChakraProvider>
    </StrictMode>,
    document.getElementById("root")
  );
});
