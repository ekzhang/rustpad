import { useEffect } from "react";
import { set_panic_hook } from "rustpad-wasm";
import Rustpad from "./rustpad";

set_panic_hook();

const WS_URI =
  (window.location.origin.startsWith("https") ? "wss://" : "ws://") +
  window.location.host +
  "/api/socket";

function App() {
  useEffect(() => {
    const rustpad = new Rustpad({
      uri: WS_URI,
      onConnected: () => console.log("connected!"),
      onDisconnected: () => console.log("disconnected!"),
    });
    return () => rustpad.dispose();
  }, []);

  return (
    <div className="container">
      <div className="row">
        <div className="one-half column" style={{ marginTop: "25%" }}>
          <h4>Chat Application</h4>
          <p>Let's send some messages!</p>
        </div>
      </div>
    </div>
  );
}

export default App;
