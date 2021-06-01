import { StrictMode } from "react";
import ReactDOM from "react-dom";
import "./styles/normalize.css";
import "./styles/skeleton.css";

// An asynchronous entry point is needed to load WebAssembly files.
import("./App").then(({ default: App }) => {
  ReactDOM.render(
    <StrictMode>
      <App />
    </StrictMode>,
    document.getElementById("root")
  );
});
