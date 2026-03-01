import { useState } from "react";
import style from "./style.module.css";
import Canvas from "./canvas";
import buttonA from "./assets/buttonA.svg";
import buttonB from "./assets/buttonB.svg";
import startSelect from "./assets/startSelect.svg";
import Button from "./button";
import Dpad from "./dpad";
import Room from "./room";
import initNode from "./init-node.ts";
import RomInput from "./rom-input.tsx";
import "./bulma.scss";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faDownload } from "@fortawesome/free-solid-svg-icons";

type Page = "game" | "settings";

function App() {
  const [node, setNode] = useState<
    { type: "ready"; value: AudioWorkletNode } | { type: "loading" }
  >();
  if (node === undefined) {
    return (
      <div className={style.center}>
        <button
          className={style.startButton}
          onClick={() => {
            setNode({ type: "loading" });
            void initNode().then((value) => {
              setNode({ type: "ready", value });
            });
          }}
        >
          🥚
        </button>
      </div>
    );
  }
  if (node.type === "loading") {
    return (
      <div className={style.center}>
        <button className={style.startButton}>🐣</button>
      </div>
    );
  }
  return <Initialized port={node.value.port} />;
}

function Initialized({ port }: { port: MessagePort }) {
  const [page, setPage] = useState<Page>("settings");

  return (
    <>
      <Settings port={port} isHidden={page !== "settings"} setPage={setPage} />
      <Game port={port} isHidden={page !== "game"} setPage={setPage} />
    </>
  );
}

function Game({
  port,
  isHidden,
  setPage,
}: {
  port: MessagePort;
  isHidden: boolean;
  setPage: (page: Page) => void;
}) {
  return (
    <div className={style.center}>
      <div className={style.content} style={{ display: isHidden ? "none" : undefined }}>
        <div className={style.screen}>
          <Canvas port={port} />
        </div>
        <div className={style.center}>
          <button
            className={style.settingsButton}
            onClick={() => {
              setPage("settings");
            }}
          >
            ⚙️
          </button>
        </div>
        <div className={style.buttonsDpadsRow}>
          <Dpad port={port} />
          <div className={style.buttons}>
            <Button style={{ marginTop: "50%" }} src={buttonB} button="b" port={port} />
            <Button src={buttonA} button="a" port={port} />
          </div>
        </div>
        <div className={style.center}>
          <div className={style.startSelectButtons}>
            <Button src={startSelect} button="select" port={port} />
            <Button src={startSelect} button="start" port={port} />
          </div>
        </div>
      </div>
    </div>
  );
}

function Settings({
  port,
  isHidden,
  setPage,
}: {
  port: MessagePort;
  isHidden: boolean;
  setPage: (page: Page) => void;
}) {
  return (
    <section className="section" style={{ display: isHidden ? "none" : undefined }}>
      <div className="container">
        <div className="field">
          <button
            className="button"
            onClick={() => {
              setPage("game");
            }}
          >
            Close settings
          </button>
        </div>
        <h1 className="title">Game</h1>
        <RomInput
          port={port}
          onLoad={() => {
            setPage("game");
          }}
        />
        <div className="field">
          <button className="button">
            <span className="icon">
              <FontAwesomeIcon icon={faDownload} />
            </span>
            <span>Download save</span>
          </button>
        </div>
        <h1 className="title">Online Multiplayer</h1>
        <Room port={port} />
      </div>
    </section>
  );
}

export default App;
