import { useEffect, useState } from "react";
import style from "./style.module.css";
import Canvas from "./canvas";
import buttonA from "./assets/buttonA.svg";
import buttonB from "./assets/buttonB.svg";
import startSelect from "./assets/startSelect.svg";
import GamepadButton from "./gamepad-button.tsx";
import Dpad from "./dpad";
import initNode from "./init-node.ts";
import RomInput from "./rom-input.tsx";
import "./bulma.scss";
import Button from "./bulma/button.tsx";
import { faArrowLeft } from "@fortawesome/free-solid-svg-icons/faArrowLeft";
import SaveSettings from "./save-settings.tsx";
import Room from "./multiplayer/room.tsx";
import type { CompatibilityMode, FromMainMessage } from "./common.ts";

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
            <GamepadButton style={{ marginTop: "50%" }} src={buttonB} button="b" port={port} />
            <GamepadButton src={buttonA} button="a" port={port} />
          </div>
        </div>
        <div className={style.center}>
          <div className={style.startSelectButtons}>
            <GamepadButton src={startSelect} button="select" port={port} />
            <GamepadButton src={startSelect} button="start" port={port} />
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
  const [mode, setMode] = useState<CompatibilityMode>("cgb-when-explicit");

  useEffect(() => {
    port.postMessage({ type: "compatibilityMode", value: mode } satisfies FromMainMessage, []);
  }, [mode, port]);

  return (
    <section className="section" style={{ display: isHidden ? "none" : undefined }}>
      <div className="container">
        <div className="field">
          <Button
            onClick={() => {
              setPage("game");
            }}
            label="Close settings"
            icon={faArrowLeft}
          />
        </div>
        <h1 className="title">Game</h1>
        <RomInput
          port={port}
          onLoad={() => {
            setPage("game");
          }}
        />
        <h5 className="title is-5">Mode</h5>
        <div className="control">
          <label className="radio">
            <input
              type="radio"
              checked={mode === "cgb-when-explicit"}
              onChange={() => {
                setMode("cgb-when-explicit");
              }}
            />{" "}
            CGB when compatiblity is explicit
          </label>{" "}
          <label className="radio">
            <input
              type="radio"
              checked={mode === "dmg-when-possible"}
              onChange={() => {
                setMode("dmg-when-possible");
              }}
            />{" "}
            DMG when possible
          </label>{" "}
          <label className="radio">
            <input
              type="radio"
              checked={mode === "always-cgb"}
              onChange={() => {
                setMode("always-cgb");
              }}
            />{" "}
            Always CGB
          </label>
        </div>
        <h1 className="title">Save</h1>
        {/* to trash the component when hidden and refresh the internal state when mounted */}
        {!isHidden && <SaveSettings />}
        <h1 className="title">Online Multiplayer</h1>
        <Room port={port} />
      </div>
    </section>
  );
}

export default App;
