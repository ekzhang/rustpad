import { useEffect, useState } from "react";
import { set_panic_hook } from "rustpad-core";

function App() {
  const [input, setInput] = useState("");
  const [socket, setSocket] = useState<WebSocket>();
  const [messages, setMessages] = useState<[number, string][]>([]);

  useEffect(() => {
    set_panic_hook();

    const uri =
      (window.location.origin.startsWith("https") ? "wss://" : "ws://") +
      window.location.host +
      "/api/socket";
    const ws = new WebSocket(uri);
    console.log("connecting...");
    ws.onopen = () => {
      console.log("connected!");
      setSocket(ws);
    };
    ws.onmessage = ({ data }) => {
      console.log("message:", data);
      setMessages((messages) => [...messages, ...JSON.parse(data)]);
    };
    ws.onclose = () => {
      console.log("disconnected!");
      setSocket(undefined);
    };
    return () => ws.close();
  }, []);

  return (
    <div className="container">
      <div className="row">
        <div className="one-half column" style={{ marginTop: "25%" }}>
          <h4>Chat Application</h4>
          <p>Let's send some messages!</p>
          <ul>
            {messages.map(([sender, message], key) => (
              <li key={key}>
                <strong>User #{sender}:</strong> {message}
              </li>
            ))}
          </ul>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              socket?.send(input);
              setInput("");
            }}
          >
            <input
              className="u-full-width"
              required
              placeholder="Hello!"
              value={input}
              onChange={(event) => setInput(event.target.value)}
            />
            <input className="button-primary" type="submit" />
          </form>
        </div>
      </div>
    </div>
  );
}

export default App;
