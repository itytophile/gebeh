import { useState } from "react";
import "../style.css";
import Canvas from "./canvas";
import buttonA from "../assets/buttonA.svg";
import buttonB from "../assets/buttonB.svg";
import startSelect from "../assets/startSelect.svg";
import Button from "./button";
import Dpad from "./dpad";
import Room from "./room";
import { type FromMainMessage } from "./common.ts";
import { getSave } from "./saves";
import initNode from "./init-node.ts";

function getTitleFromRom(rom: Uint8Array): string {
  const title = rom.slice(0x134, 0x143);

  let endZeroPos = title.indexOf(0);
  if (endZeroPos === -1) {
    endZeroPos = title.length;
  }

  const decoder = new TextDecoder("utf-8", { fatal: true });
  return decoder.decode(title.slice(0, endZeroPos));
}

function App() {
  const [node, setNode] = useState<AudioWorkletNode>();
  if (!node) {
    return (
      <button
        onClick={async () => {
          setNode(await initNode());
        }}
      >
        Turn on
      </button>
    );
  }
  return <AppInner port={node.port} />;
}

function AppInner({ port }: { port: MessagePort }) {
  return (
    <div className="content">
      <div className="screen">
        <div className="toolbar">
          <input
            type="file"
            onChange={async (event) => {
              const file = event.target.files?.item(0);
              if (file) {
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
              } else {
                console.error("Can't load file");
              }
            }}
          />
          <Room port={port} />
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
