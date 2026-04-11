import { useState, useEffect } from "react";
import type { FromNodeMessage, FromMainMessage } from "./common";
import Button from "./bulma/button";
import { faRocket, faPlug, faRotateLeft } from "@fortawesome/free-solid-svg-icons";

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

function CreatedRoom({ port }: { port: MessagePort; isWebRtcEnabled: boolean }) {
  const [status, setStatus] = useState<
    | { type: "loading" }
    | { type: "closed" }
    | { type: "ready"; room: string; ws: WebSocket }
    | { type: "waiting"; room: string }
  >({ type: "loading" });

  useEffect(() => {
    const ws = new WebSocket(`${globalThis.location.protocol}//${globalThis.location.host}/ws`);
    ws.binaryType = "arraybuffer";

    let state: { type: "waitName" } | { type: "waitGuest"; room: string } = {
      type: "waitName",
    };

    const onMessageForInitialization = (message: MessageEvent<unknown>) => {
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
          ws.removeEventListener("message", onMessageForInitialization);
          setStatus({ type: "ready", room: state.room, ws });
          break;
        }
      }
    };

    ws.addEventListener("message", onMessageForInitialization);

    ws.addEventListener("close", () => {
      setStatus({ type: "closed" });
    });

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

  return (
    <>
      <WebSocketMultiplayer port={port} ws={status.ws} />
      <Button label="Connected 🐣🐔" />
    </>
  );
}

function JoinedRoom({
  room,
  port,
}: {
  room: string;
  port: MessagePort;
  isWebRtcEnabled: boolean;
}) {
  const [status, setStatus] = useState<
    { type: "loading" } | { type: "ready"; ws: WebSocket } | { type: "closed" }
  >({ type: "loading" });
  useEffect(() => {
    const ws = new WebSocket(
      `${globalThis.location.protocol}//${globalThis.location.host}/ws?room=${room}`,
    );
    ws.binaryType = "arraybuffer";
    ws.addEventListener("open", () => {
      setStatus({ type: "ready", ws });
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

  return (
    <>
      <WebSocketMultiplayer port={port} ws={status.ws} />
      <Button label="Connected 🐣🐔" />
    </>
  );
}

export default Room;

function WebSocketMultiplayer({ port, ws }: { port: MessagePort; ws: WebSocket }): undefined {
  useEffect(() => {
    const portListener = ({ data }: MessageEvent<FromNodeMessage>) => {
      if (data.type === "serial") {
        ws.send(data.buffer);
      }
    };
    port.addEventListener("message", portListener);
    const wsListener = (message: MessageEvent<unknown>) => {
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
    };
    ws.addEventListener("message", wsListener);
    port.postMessage({
      type: "serialConnected",
    } satisfies FromMainMessage);

    return () => {
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
      port.removeEventListener("message", portListener);
      ws.removeEventListener("message", wsListener);
    };
  }, [ws, port]);
}

// function WebRtc({ port, ws }: { port: MessagePort; ws: WebSocket }) {
//   useEffect(() => {
//     const pc = new RTCPeerConnection({
//       iceServers: [
//         {
//           urls: "stun:localhost:3478",
//         },
//       ],
//     });

//     pc.createDataChannel("prout");

//     pc.addEventListener("icecandidate", (event) => {
//       if (event.candidate) {
//         console.log(event.candidate);
//         ws.send(JSON.stringify(event.candidate));
//       }
//     });
//     pc.addEventListener("connectionstatechange", (event) => {
//       console.log({ connectionstate: event });
//     });

//     void pc.createOffer().then((offer) => {
//       console.log("Offer created:", offer);
//       return pc.setLocalDescription(offer);
//     });

//     return () => {
//       pc.close();
//     };
//   }, [port, ws]);
// }
