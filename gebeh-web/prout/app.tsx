import { useEffect, useRef } from "react";
import { initCanvas, onLoadFile } from ".";
import "../style.css";

function App() {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current) {
      throw new TypeError("No canvas");
    }
    initCanvas(canvas.current);
  }, []);

  return (
    <div className="content">
      <div className="screen">
        <div id="toolbar" className="toolbar">
          <input
            type="file"
            id="rom-input"
            onChange={(event) => {
              const file = event.target.files?.item(0);
              const context = canvas.current?.getContext("2d");
              if (file && context) {
                void onLoadFile(file, context);
              } else {
                console.error("Can't load file");
              }
            }}
          />
          <div className="flex-row" id="room">
            <div className="flex-row">
              <input type="text" placeholder="Room to join" id="roomInput" />
              <button id="joinRoomBtn">Join room</button>
            </div>
            <button id="createRoomBtn">Create room</button>
          </div>
        </div>
        <canvas
          ref={canvas}
          id="canvas"
          tabIndex={1}
          width="160"
          height="144"
        ></canvas>
      </div>
      <div className="buttons-dpads-row">
        <img className="interactive" src="assets/dpad.svg" id="dpad" />
        <div className="buttons">
          <img
            className="interactive"
            style={{ marginTop: "50%" }}
            src="assets/buttonB.svg"
            id="buttonB"
          />
          <img className="interactive" src="assets/buttonA.svg" id="buttonA" />
        </div>
      </div>
      <div className="center">
        <div className="start-select-buttons">
          <img
            className="interactive"
            src="assets/startSelect.svg"
            id="buttonSelect"
          />
          <img
            className="interactive"
            src="assets/startSelect.svg"
            id="buttonStart"
          />
        </div>
      </div>
    </div>
  );
}

export default App;
