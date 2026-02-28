import { useState } from "react";
import "../style.css";
import Canvas from "./canvas";
import buttonA from "../assets/buttonA.svg";
import buttonB from "../assets/buttonB.svg";
import startSelect from "../assets/startSelect.svg";
import Button from "./button";
import Dpad from "./dpad";
import { CreatedRoom, JoinedRoom } from "./room";
import { AUDIO_PROCESSOR_NAME, type FromMainMessage, type FromNodeMessage } from "./common.ts";
import { getSave, writeSave } from "./saves";
import workletURL from "./worklet.ts?worker&url";
import wasm from "../pkg/gebeh_web_bg.wasm?url";

function getTitleFromRom(rom: Uint8Array): string {
  const title = rom.slice(0x134, 0x143);

  let endZeroPos = title.indexOf(0);
  if (endZeroPos === -1) {
    endZeroPos = title.length;
  }

  const decoder = new TextDecoder("utf-8", { fatal: true });
  return decoder.decode(title.slice(0, endZeroPos));
}

const onLoadFile = async (file: File, port: MessagePort) => {
  const bytes = new Uint8Array(await file.arrayBuffer());

  const save = await getSave(getTitleFromRom(new Uint8Array(bytes)));
  port.postMessage(
    {
      type: "rom",
      bytes,
      save,
    } satisfies FromMainMessage,
    save ? [bytes.buffer, save.buffer] : [bytes.buffer],
  );
};

function App() {
  const [node, setNode] = useState<AudioWorkletNode>();
  if (!node) {
    return (
      <button
        onClick={async () => {
          const audioContext = new AudioContext();
          await audioContext.audioWorklet.addModule(workletURL);
          const node = new AudioWorkletNode(audioContext, AUDIO_PROCESSOR_NAME, {
            outputChannelCount: [2],
          });
          const { port } = node;
          // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
          port.addEventListener("message", async ({ data }: MessageEvent<FromNodeMessage>) => {
            switch (data.type) {
              case "ready": {
                // ready
                setNode(node);
                break;
              }
              case "wasm": {
                console.log("Sending wasm");
                // https://github.com/wasm-bindgen/wasm-bindgen/blob/9ffc52c8d29f006cadf669dcfce6b6f74d308194/examples/synchronous-instantiation/index.html
                void fetch(wasm)
                  .then((response) => response.bytes())
                  .then((bytes) => {
                    port.postMessage({ type: "wasm", bytes } satisfies FromMainMessage, [
                      bytes.buffer,
                    ]);
                  });
                break;
              }
              case "save": {
                await writeSave(data.title, data.buffer);
              }
            }
          });
          document.addEventListener("visibilitychange", () => {
            if (document.visibilityState == "visible") {
              port.postMessage({
                type: "enableMessages",
              } satisfies FromMainMessage);
            } else {
              port.postMessage({
                type: "disableMessages",
              } satisfies FromMainMessage);
            }
          });
          port.start();
          node.connect(audioContext.destination);
        }}
      >
        Turn on
      </button>
    );
  }
  return <AppInner port={node.port} />;
}

function AppInner({ port }: { port: MessagePort }) {
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
                await onLoadFile(file, port);
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
          {room.type === "joined" && <JoinedRoom port={port} room={room.name} />}
          {room.type === "created" && <CreatedRoom port={port} />}
        </div>
        {<Canvas port={port} />}
      </div>
      <div className="buttons-dpads-row">
        <Dpad port={port} />
        <div className="buttons">
          <Button style={{ marginTop: "50%" }} src={buttonB} button="b" port={port} />
          <Button src={buttonA} button="a" port={port} />
        </div>
      </div>
      <div className="center">
        <div className="start-select-buttons">
          <Button src={startSelect} button="select" port={port} />
          <Button src={startSelect} button="start" port={port} />
        </div>
      </div>
    </div>
  );
}

export default App;
