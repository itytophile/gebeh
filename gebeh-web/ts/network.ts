import { FromNodeMessage } from "./common";

const roomInput = document.getElementById("roomInput");

if (!(roomInput instanceof HTMLInputElement)) {
  throw new TypeError("roomInput is not an input");
}

const createRoomButton = document.getElementById("createRoomBtn");

if (!(createRoomButton instanceof HTMLButtonElement)) {
  throw new TypeError("createRoomBtn is not an input");
}

const joinRoomButton = document.getElementById("joinRoomBtn");

if (!(joinRoomButton instanceof HTMLButtonElement)) {
  throw new TypeError("joinRoomButton is not an input");
}

const roomDiv = document.getElementById("room");

if (!(roomDiv instanceof HTMLDivElement)) {
  throw new TypeError("roomDiv is not a div");
}

const CLOSE_MESSAGE = "Room closed 🍗🍗";

function getReadyRoomMessage(room: string) {
  return `${room} 🐣🐔`;
}

export const addNetwork = (port: MessagePort) => {
  createRoomButton.addEventListener("click", () => {
    const ws = new WebSocket("http://localhost:8080");
    ws.binaryType = "arraybuffer";
    ws.addEventListener("open", () => {
      console.log("host!");
      port.addEventListener(
        "message",
        ({ data }: MessageEvent<FromNodeMessage>) => {
          if (data.type === "serial") {
            ws.send(new Uint8Array([data.byte]));
          }
        },
      );
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
          roomDiv.textContent = `${message.data} 🥚🐔`;
          state = { type: "waitGuest", room: message.data };
          break;
        }
        case "waitGuest": {
          roomDiv.textContent = getReadyRoomMessage(state.room);
          state = { type: "done" };
          break;
        }
        case "done": {
          if (!(message.data instanceof ArrayBuffer)) {
            throw new TypeError("Only binary messages are accepted");
          }
          port.postMessage({
            type: "serial",
            byte: new DataView(message.data).getUint8(0),
          } satisfies FromNodeMessage);
          break;
        }
      }
    });
    ws.addEventListener("close", () => {
      roomDiv.textContent = CLOSE_MESSAGE;
    });
  });

  joinRoomButton.addEventListener("click", () => {
    const room = roomInput.value.trim();
    const ws = new WebSocket(`http://localhost:8080?room=${room}`);
    ws.binaryType = "arraybuffer";
    ws.addEventListener("open", () => {
      roomDiv.textContent = getReadyRoomMessage(room);
      console.log("guest!");
      port.addEventListener(
        "message",
        ({ data }: MessageEvent<FromNodeMessage>) => {
          if (data.type === "serial") {
            ws.send(new Uint8Array([data.byte]));
          }
        },
      );
    });
    ws.addEventListener("message", (message) => {
      if (!(message.data instanceof ArrayBuffer)) {
        console.log(message.data);
        throw new TypeError("Only binary messages are accepted");
      }
      port.postMessage({
        type: "serial",
        byte: new DataView(message.data).getUint8(0),
      } satisfies FromNodeMessage);
    });
    ws.addEventListener("close", () => {
      roomDiv.textContent = CLOSE_MESSAGE;
    });
  });
};
