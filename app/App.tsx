import { useState } from "react";

function App() {
  const [count, setCount] = useState(0);

  return (
    <div className="container">
      <div className="row">
        <div className="one-half column" style={{ marginTop: "25%" }}>
          <h4>Skeleton + React + Vite</h4>
          <p>
            This index.html page is a placeholder with the CSS, font and
            favicon. It's just waiting for you to add some content! If you need
            some help hit up the{" "}
            <a href="http://www.getskeleton.com">Skeleton documentation</a>.
          </p>
          <button onClick={() => setCount((count) => count + 1)}>
            count is: {count}
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
