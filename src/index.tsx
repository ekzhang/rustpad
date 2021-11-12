import { StrictMode } from "react";
import ReactDOM from "react-dom";
import { ChakraProvider } from "@chakra-ui/react";
import init, { set_panic_hook } from "rustpad-wasm";
import App from "./App";
import "./index.css";

init().then(() => {
  set_panic_hook();
  ReactDOM.render(
    <StrictMode>
      <ChakraProvider>
        <App />
      </ChakraProvider>
    </StrictMode>,
    document.getElementById("root")
  );
});
