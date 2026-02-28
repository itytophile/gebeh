import { useState } from "react";
import { onLoadFile } from ".";
import "../style.css";
import Canvas from "./canvas";

function App() {
  const [node, setNode] = useState<AudioWorkletNode>();

  return (
    <div className="content">
      <div className="screen">
        <div className="toolbar">
          <input
            type="file"
            onChange={async (event) => {
              const file = event.target.files?.item(0);
              if (file) {
                const node = await onLoadFile(file);
                setNode(node);
              } else {
                console.error("Can't load file");
              }
            }}
          />
          <div className="flex-row">
            <div className="flex-row">
              <input type="text" placeholder="Room to join" />
              <button>Join room</button>
            </div>
            <button>Create room</button>
          </div>
        </div>
        {node?.port && <Canvas port={node.port} />}
      </div>
      <div className="buttons-dpads-row">
        <img className="interactive" src="assets/dpad.svg" />
        <div className="buttons">
          <img
            className="interactive"
            style={{ marginTop: "50%" }}
            src="assets/buttonB.svg"
          />
          <img className="interactive" src="assets/buttonA.svg" />
        </div>
      </div>
      <div className="center">
        <div className="start-select-buttons">
          <img className="interactive" src="assets/startSelect.svg" />
          <img className="interactive" src="assets/startSelect.svg" />
        </div>
      </div>
    </div>
  );
}

export default App;
