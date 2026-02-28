import { useState } from "react";
import "../style.css";
import Canvas from "./canvas";
import buttonA from "../assets/buttonA.svg";
import buttonB from "../assets/buttonB.svg";
import startSelect from "../assets/startSelect.svg";
import Button from "./button";
import Dpad from "./dpad";
import Room from "./room";
import initNode from "./init-node.ts";
import RomInput from "./rom-input.tsx";

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
  return <Initialized port={node.port} />;
}

function Initialized({ port }: { port: MessagePort }) {
  return (
    <div className="content">
      <div className="screen">
        <div className="toolbar">
          <RomInput port={port} />
          <Room port={port} />
        </div>
        <Canvas port={port} />
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
