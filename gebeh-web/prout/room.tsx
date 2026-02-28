import { useState, useEffect } from "react";
import type { FromNodeMessage, FromMainMessage } from "./common";

const CLOSE_MESSAGE = "Room closed 🍗🍗";

function getReadyRoomMessage(room: string) {
  return `${room} 🐣🐔`;
}

export function CreatedRoom({ port }: { port: MessagePort }) {
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

export function JoinedRoom({
  room,
  port,
}: {
  room: string;
  port: MessagePort;
}) {
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
