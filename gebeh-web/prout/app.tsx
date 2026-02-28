import { useEffect, useState } from "react";
import { onLoadFile } from ".";
import "../style.css";
import Canvas from "./canvas";
import type { FromMainMessage, FromNodeMessage } from "./common";

function App() {
  const [node, setNode] = useState<AudioWorkletNode>();
  const [room, setRoom] = useState<
    | { type: "input"; value: string }
    | { type: "created" }
    | { type: "joined"; name: string }
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
          {room.type === "joined" && node?.port && (
            <JoinedRoom port={node.port} room={room.name} />
          )}
          {room.type === "created" && node?.port && (
            <CreatedRoom port={node.port} />
          )}
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

const CLOSE_MESSAGE = "Room closed 🍗🍗";

function getReadyRoomMessage(room: string) {
  return `${room} 🐣🐔`;
}

function CreatedRoom({ port }: { port: MessagePort }) {
  const [status, setStatus] = useState("");

  useEffect(() => {
    const ws = new WebSocket(
      `${globalThis.location.protocol}//${globalThis.location.host}/ws`,
    );
    ws.binaryType = "arraybuffer";
    const portListener = ({ data }: MessageEvent<FromNodeMessage>) => {
      if (data.type === "serial") {
        ws.send(data.buffer);
      }
    };
    ws.addEventListener("open", () => {
      console.log("host!");
      port.addEventListener("message", portListener);
    });

    let state:
      | { type: "waitName" }
      | { type: "waitGuest"; room: string }
      | { type: "done" } = {
      type: "waitName",
    };

    ws.addEventListener("message", (message) => {
      switch (state.type) {
        case "waitName": {
          if (typeof message.data !== "string") {
            throw new TypeError("First message must be the room name");
          }
          setStatus(`${message.data} 🥚🐔`);
          state = { type: "waitGuest", room: message.data };
          break;
        }
        case "waitGuest": {
          setStatus(getReadyRoomMessage(state.room));
          state = { type: "done" };
          port.postMessage({
            type: "serialConnected",
          } satisfies FromMainMessage);
          break;
        }
        case "done": {
          if (!(message.data instanceof ArrayBuffer)) {
            throw new TypeError("Only binary messages are accepted");
          }
          port.postMessage(
            {
              type: "serial",
              buffer: new Uint8Array(message.data),
            } satisfies FromMainMessage,
            [message.data],
          );
          break;
        }
      }
    });
    ws.addEventListener("close", () => {
      setStatus(CLOSE_MESSAGE);
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    });

    return () => {
      ws.close();
      port.removeEventListener("message", portListener);
    };
  }, [port]);

  return status;
}

function JoinedRoom({ room, port }: { room: string; port: MessagePort }) {
  const [status, setStatus] = useState("");
  useEffect(() => {
    const ws = new WebSocket(
      `${globalThis.location.protocol}//${globalThis.location.host}/ws?room=${room}`,
    );
    ws.binaryType = "arraybuffer";
    const portListener = ({ data }: MessageEvent<FromNodeMessage>) => {
      if (data.type === "serial") {
        ws.send(new Uint8Array(data.buffer));
      }
    };
    ws.addEventListener("open", () => {
      setStatus(getReadyRoomMessage(room));
      console.log("guest!");
      port.postMessage({ type: "serialConnected" } satisfies FromMainMessage);
      port.addEventListener("message", portListener);
    });
    ws.addEventListener("message", (message) => {
      if (!(message.data instanceof ArrayBuffer)) {
        console.log(message.data);
        throw new TypeError("Only binary messages are accepted");
      }
      port.postMessage({
        type: "serial",
        buffer: new Uint8Array(message.data),
      } satisfies FromNodeMessage);
    });
    ws.addEventListener("close", () => {
      setStatus(CLOSE_MESSAGE);
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    });

    return () => {
      ws.close();
      port.removeEventListener("message", portListener);
    };
  }, [port, room]);
  return status;
}

export default App;
