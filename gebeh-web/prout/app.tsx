import { useState } from "react";
import { onLoadFile } from ".";
import "../style.css";
import Canvas from "./canvas";
import buttonA from "../assets/buttonA.svg";
import buttonB from "../assets/buttonB.svg";
import startSelect from "../assets/startSelect.svg";
import Button from "./button";
import Dpad from "./dpad";
import { CreatedRoom, JoinedRoom } from "./room";

function App() {
  const [node, setNode] = useState<AudioWorkletNode>();
  const [room, setRoom] = useState<
    { type: "input"; value: string } | { type: "created" } | { type: "joined"; name: string }
  >({ type: "input", value: "" });

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
          {room.type === "input" && (
            <div className="flex-row">
              <div className="flex-row">
                <input
                  type="text"
                  placeholder="Room to join"
                  value={room.value}
                  onChange={(event) => {
                    setRoom({ type: "input", value: event.target.value });
                  }}
                />
                <button
                  onClick={() => {
                    setRoom({ type: "joined", name: room.value });
                  }}
                >
                  Join room
                </button>
              </div>
              <button
                onClick={() => {
                  setRoom({ type: "created" });
                }}
              >
                Create room
              </button>
            </div>
          )}
          {room.type === "joined" && node?.port && <JoinedRoom port={node.port} room={room.name} />}
          {room.type === "created" && node?.port && <CreatedRoom port={node.port} />}
        </div>
        {node?.port && <Canvas port={node.port} />}
      </div>
      {node?.port && (
        <>
          <div className="buttons-dpads-row">
            <Dpad port={node.port} />
            <div className="buttons">
              <Button style={{ marginTop: "50%" }} src={buttonB} button="b" port={node.port} />
              <Button src={buttonA} button="a" port={node.port} />
            </div>
          </div>
          <div className="center">
            <div className="start-select-buttons">
              <Button src={startSelect} button="select" port={node.port} />
              <Button src={startSelect} button="start" port={node.port} />
            </div>
          </div>
        </>
      )}
    </div>
  );
}

export default App;
