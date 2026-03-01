import { useState, useEffect } from "react";
import type { FromNodeMessage, FromMainMessage } from "./common";
import Button from "./bulma/button";

function Room({ port }: { port: MessagePort }) {
  const [room, setRoom] = useState<
    { type: "input"; value: string } | { type: "created" } | { type: "joined"; name: string }
  >({ type: "input", value: "" });

  if (room.type === "input") {
    return (
      <>
        <div className="field">
          <Button
            onClick={() => {
              setRoom({ type: "created" });
            }}
            label="Create room"
            color="is-success"
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
            />
          </div>
        </div>
      </>
    );
  }

  if (room.type === "created") {
    return <CreatedRoom port={port} />;
  }

  return <JoinedRoom port={port} room={room.name} />;
}

function CreatedRoom({ port }: { port: MessagePort }) {
  const [status, setStatus] = useState<
    | { type: "loading" }
    | { type: "closed" }
    | { type: "ready"; room: string }
    | { type: "waiting"; room: string }
  >({ type: "loading" });

  useEffect(() => {
    const ws = new WebSocket(`${globalThis.location.protocol}//${globalThis.location.host}/ws`);
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

    let state: { type: "waitName" } | { type: "waitGuest"; room: string } | { type: "done" } = {
      type: "waitName",
    };

    ws.addEventListener("message", (message) => {
      switch (state.type) {
        case "waitName": {
          if (typeof message.data !== "string") {
            throw new TypeError("First message must be the room name");
          }
          setStatus({ type: "waiting", room: message.data });
          state = { type: "waitGuest", room: message.data };
          break;
        }
        case "waitGuest": {
          setStatus({ type: "ready", room: state.room });
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
      setStatus({ type: "closed" });
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    });

    return () => {
      ws.close();
      port.removeEventListener("message", portListener);
    };
  }, [port]);

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

  return <Button label="Connected 🐣🐔" />;
}

function JoinedRoom({ room, port }: { room: string; port: MessagePort }) {
  const [status, setStatus] = useState<"loading" | "ready" | "closed">("loading");
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
      setStatus("ready");
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
      setStatus("closed");
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    });

    return () => {
      ws.close();
      port.removeEventListener("message", portListener);
    };
  }, [port, room]);

  if (status === "loading") {
    return <Button label="Loading..." />;
  }

  if (status === "closed") {
    return <Button label="Room closed 🍗🍗" />;
  }

  return <Button label="Connected 🐣🐔" />;
}

export default Room;
