import { StrictMode } from "react";
import ReactDOM from "react-dom";
import { ChakraProvider } from "@chakra-ui/react";
import "./index.css";

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
