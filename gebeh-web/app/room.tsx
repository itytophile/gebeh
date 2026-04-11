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

function CreatedRoom({ port, isWebRtcEnabled }: { port: MessagePort; isWebRtcEnabled: boolean }) {
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

  return isWebRtcEnabled ? (
    <WebRtcMultiplayer port={port} ws={status.ws} />
  ) : (
    <WebSocketMultiplayer port={port} ws={status.ws} />
  );
}

export default Room;

function WebSocketMultiplayer({ port, ws }: { port: MessagePort; ws: WebSocket }) {
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

  return <Button label="Connected 🐣🐔" />;
}

function WebRtcMultiplayer({ port, ws }: { port: MessagePort; ws: WebSocket }) {
  const [channel, setChannel] = useState<RTCDataChannel>();

  useEffect(() => {
    const pc = new RTCPeerConnection({
      iceServers: [
        {
          urls: "stun:stun.l.google.com:19302",
        },
      ],
    });

    const icecandidateListener = (event: RTCPeerConnectionIceEvent) => {
      if (event.candidate) {
        const text = JSON.stringify({ candidate: event.candidate });
        ws.send(text);
      }
    };

    pc.addEventListener("icecandidate", icecandidateListener);
    pc.addEventListener("connectionstatechange", console.log);

    const wsListener = async (message: MessageEvent<unknown>) => {
      if (typeof message.data != "string") {
        throw new TypeError("Only text messages are accepted");
      }
      const parsed = JSON.parse(message.data) as
        | { candidate: RTCIceCandidate }
        | { offer: RTCSessionDescriptionInit };
      if ("offer" in parsed) {
        pc.ondatachannel = (event) => {
          console.log("new channel", event.channel.label);
          const receiveChannel = event.channel;
          receiveChannel.binaryType = "arraybuffer";
          receiveChannel.addEventListener("open", () => {
            port.postMessage({
              type: "serialConnected",
            } satisfies FromMainMessage);
          });
          setChannel(receiveChannel);
        };
        console.log("offer received");
        await pc.setRemoteDescription(new RTCSessionDescription(parsed.offer));
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        ws.send(JSON.stringify({ answer }));
        return;
      }
      console.log("un candidat!!");
      await pc.addIceCandidate(parsed.candidate);
    };

    ws.addEventListener("message", wsListener);

    return () => {
      console.log("ras le bol");
      pc.close();
      ws.removeEventListener("message", wsListener);
    };
  }, [ws, port]);

  return channel ? <DataChannelHandler channel={channel} port={port} /> : "rtc wait";
}

function WebRtcMultiplayerOfferer({ port, ws }: { port: MessagePort; ws: WebSocket }) {
  const [channel, setChannel] = useState<RTCDataChannel>();

  useEffect(() => {
    const pc = new RTCPeerConnection({
      iceServers: [
        {
          urls: "stun:stun.l.google.com:19302",
        },
      ],
    });

    const icecandidateListener = (event: RTCPeerConnectionIceEvent) => {
      if (event.candidate) {
        const text = JSON.stringify({ candidate: event.candidate });
        ws.send(text);
      }
    };

    pc.addEventListener("icecandidate", icecandidateListener);
    pc.addEventListener("connectionstatechange", console.log);

    // TODO faire gaffe à la concurrence pendant la connexion au cas où un des joueurs balance des messages trop tôt
    const dataChannel = pc.createDataChannel("prout");
    dataChannel.binaryType = "arraybuffer";
    dataChannel.addEventListener("open", () => {
      port.postMessage({
        type: "serialConnected",
      } satisfies FromMainMessage);
    });

    const wsListener = async (message: MessageEvent<unknown>) => {
      if (typeof message.data != "string") {
        throw new TypeError("Only text messages are accepted");
      }
      const parsed = JSON.parse(message.data) as
        | { candidate: RTCIceCandidate }
        | { answer: RTCSessionDescriptionInit };
      if ("answer" in parsed) {
        console.log("answer received");
        await pc.setRemoteDescription(new RTCSessionDescription(parsed.answer));
        setChannel(dataChannel);

        return;
      }
      console.log("un candidat!!");
      await pc.addIceCandidate(parsed.candidate);
    };

    ws.addEventListener("message", wsListener);

    void pc.createOffer().then(async (offer) => {
      console.log("Offer created:", offer);
      await pc.setLocalDescription(offer);
      ws.send(JSON.stringify({ offer }));
    });

    return () => {
      console.log("ras le bol");
      pc.close();
      ws.removeEventListener("message", wsListener);
    };
  }, [ws, port]);

  return channel ? <DataChannelHandler channel={channel} port={port} /> : "rtc wait";
}

function DataChannelHandler({ channel, port }: { channel: RTCDataChannel; port: MessagePort }) {
  useEffect(() => {
    const portListener = ({ data }: MessageEvent<FromNodeMessage>) => {
      if (data.type === "serial") {
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-arguments
        channel.send(data.buffer as Uint8Array<ArrayBuffer>);
      }
    };

    const onWebRtcMessage = (message: MessageEvent<unknown>) => {
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

    port.addEventListener("message", portListener);

    channel.addEventListener("message", onWebRtcMessage);

    return () => {
      console.log("je close sa mère la chienne");
      channel.close();
      port.removeEventListener("message", portListener);
    };
  }, [channel, port]);

  return "rtc connected";
}

async function* websocketGenerator(ws: WebSocket): AsyncGenerator<MessageEvent<unknown>> {
  const queue: MessageEvent<unknown>[] = [];
  let resolve: undefined | ((value: MessageEvent<unknown>) => void);

  ws.addEventListener("message", (event) => {
    if (resolve) {
      resolve(event);
      resolve = undefined;
    } else {
      queue.push(event);
    }
  });

  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  while (true) {
    yield (
      queue.shift() ??
        (await new Promise<MessageEvent<unknown>>((resolve0) => (resolve = resolve0)))
    );
  }
}
