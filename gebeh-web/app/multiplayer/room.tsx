import { useState, useEffect } from "react";
import Button from "../bulma/button";
import { faRocket, faPlug, faRotateLeft } from "@fortawesome/free-solid-svg-icons";
import { WebRtcMultiplayer, WebRtcMultiplayerOfferer } from "./webrtc";
import {
  getBinaryMessage,
  getTextMessage,
  websocketGenerator,
  type WsAndMessages,
} from "./ws-helpers";
import WebSocketMultiplayer from "./ws";

function Room({ port }: { port: MessagePort }) {
  const [room, setRoom] = useState<
    { type: "input"; value: string } | { type: "created" } | { type: "joined"; name: string }
  >({ type: "input", value: "" });
  const [isWebRtcEnabled, setIsWebRtcEnabled] = useState(false);

  if (room.type === "input") {
    return (
      <>
        <div className="field">
          <label className="checkbox">
            <input
              type="checkbox"
              checked={isWebRtcEnabled}
              onChange={() => {
                setIsWebRtcEnabled((a) => !a);
              }}
            />
            {" Enable WebRTC"}
          </label>
        </div>
        <div className="field">
          <Button
            onClick={() => {
              setRoom({ type: "created" });
            }}
            label="Create room"
            color="is-success"
            icon={faRocket}
          />
        </div>
        <div className="field has-addons">
          <div className="control">
            <input
              className="input"
              type="text"
              placeholder="Room to join"
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  setRoom({ type: "joined", name: room.value });
                }
              }}
              onChange={(event) => {
                setRoom({ type: "input", value: event.target.value });
              }}
            />
          </div>
          <div className="control">
            <Button
              label="Join room"
              color="is-info"
              onClick={() => {
                setRoom({ type: "joined", name: room.value });
              }}
              icon={faPlug}
            />
          </div>
        </div>
      </>
    );
  }

  const button =
    room.type === "created" ? (
      <CreatedRoom port={port} isWebRtcEnabled={isWebRtcEnabled} />
    ) : (
      <JoinedRoom port={port} room={room.name} isWebRtcEnabled={isWebRtcEnabled} />
    );

  return (
    <div className="field has-addons">
      <div className="control">{button}</div>
      <div className="control">
        <Button
          label="Reset"
          color="is-warning"
          icon={faRotateLeft}
          onClick={() => {
            setRoom({ type: "input", value: "" });
          }}
        />
      </div>
    </div>
  );
}

function CreatedRoom({ port, isWebRtcEnabled }: { port: MessagePort; isWebRtcEnabled: boolean }) {
  const [status, setStatus] = useState<
    | { type: "loading" }
    | { type: "closed" }
    | {
        type: "ready";
        room: string;
        ws: WsAndMessages;
      }
    | { type: "waiting"; room: string }
  >({ type: "loading" });

  useEffect(() => {
    const ws = new WebSocket(`${globalThis.location.protocol}//${globalThis.location.host}/ws`);
    ws.binaryType = "arraybuffer";
    const messages = websocketGenerator(ws);

    ws.addEventListener("close", () => {
      setStatus({ type: "closed" });
    });

    void (async () => {
      const room = await getTextMessage(messages);
      if (!room) {
        return;
      }
      setStatus({ type: "waiting", room });
      if (!(await getBinaryMessage(messages))) {
        return;
      }
      // empty message, it means that guest is here
      setStatus({ type: "ready", room, ws: { inner: ws, messages } });
    })();

    return () => {
      ws.close();
    };
  }, []);

  if (status.type === "loading") {
    return <Button label="Loading..." />;
  }

  if (status.type === "closed") {
    return <Button label="Room closed 🍗🍗" />;
  }

  if (status.type === "waiting") {
    return (
      <Button
        label={`${status.room} 🥚🐔`}
        onClick={() => {
          void navigator.clipboard.writeText(status.room);
        }}
      />
    );
  }

  return isWebRtcEnabled ? (
    <WebRtcMultiplayerOfferer port={port} ws={status.ws} />
  ) : (
    <WebSocketMultiplayer port={port} ws={status.ws} />
  );
}

function JoinedRoom({
  room,
  port,
  isWebRtcEnabled,
}: {
  room: string;
  port: MessagePort;
  isWebRtcEnabled: boolean;
}) {
  const [status, setStatus] = useState<
    { type: "loading" } | { type: "ready"; ws: WsAndMessages } | { type: "closed" }
  >({ type: "loading" });
  useEffect(() => {
    const ws = new WebSocket(
      `${globalThis.location.protocol}//${globalThis.location.host}/ws?room=${room}`,
    );
    ws.binaryType = "arraybuffer";
    const messages = websocketGenerator(ws);
    ws.addEventListener("open", () => {
      setStatus({ type: "ready", ws: { inner: ws, messages } });
    });
    ws.addEventListener("close", () => {
      setStatus({ type: "closed" });
    });

    return () => {
      ws.close();
    };
  }, [room]);

  if (status.type === "loading") {
    return <Button label="Loading..." />;
  }

  if (status.type === "closed") {
    return <Button label="Room closed 🍗🍗" />;
  }

  return isWebRtcEnabled ? (
    <WebRtcMultiplayer port={port} ws={status.ws} />
  ) : (
    <WebSocketMultiplayer port={port} ws={status.ws} />
  );
}

export default Room;
