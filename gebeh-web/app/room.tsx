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

async function getBinaryMessage(
  messages: AsyncGenerator<MessageEvent<unknown>>,
): Promise<ArrayBuffer | undefined> {
  const result = await messages.next();
  if (result.done) {
    return undefined;
  }
  if (!(result.value.data instanceof ArrayBuffer)) {
    throw new TypeError("First message must be the room name");
  }
  return result.value.data;
}

async function getTextMessage(
  messages: AsyncGenerator<MessageEvent<unknown>>,
): Promise<string | undefined> {
  const result = await messages.next();
  if (result.done) {
    return undefined;
  }
  if (typeof result.value.data !== "string") {
    throw new TypeError("First message must be the room name");
  }
  return result.value.data;
}

function CreatedRoom({ port, isWebRtcEnabled }: { port: MessagePort; isWebRtcEnabled: boolean }) {
  const [status, setStatus] = useState<
    | { type: "loading" }
    | { type: "closed" }
    | {
        type: "ready";
        room: string;
        ws: WebSocket;
        messages: AsyncGenerator<MessageEvent<unknown>>;
      }
    | { type: "waiting"; room: string }
  >({ type: "loading" });

  useEffect(() => {
    const ws = new WebSocket(`${globalThis.location.protocol}//${globalThis.location.host}/ws`);
    ws.binaryType = "arraybuffer";

    ws.addEventListener("close", () => {
      setStatus({ type: "closed" });
    });

    const messages = websocketGenerator(ws);

    void (async () => {
      const room = await getTextMessage(messages);
      setStatus({ type: "waiting", room });
      await getBinaryMessage(messages);
      // empty message, it means that guest is here
      setStatus({ type: "ready", room, ws, messages });
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
    | { type: "loading" }
    | { type: "ready"; ws: WebSocket; messages: AsyncGenerator<MessageEvent<unknown>> }
    | { type: "closed" }
  >({ type: "loading" });
  useEffect(() => {
    const ws = new WebSocket(
      `${globalThis.location.protocol}//${globalThis.location.host}/ws?room=${room}`,
    );
    ws.binaryType = "arraybuffer";
    const messages = websocketGenerator(ws);
    ws.addEventListener("open", () => {
      setStatus({ type: "ready", ws, messages });
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

function WebRtcMultiplayer({ port, ws }: { port: MessagePort; ws: WsAndMessages }) {
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
        ws.inner.send(text);
      }
    };

    pc.addEventListener("icecandidate", icecandidateListener);

    void (async () => {
      while (true) {
        const text = await getTextMessage(ws.messages);
        if (!text) {
          return;
        }
        const parsed = JSON.parse(text) as
          | { offer: RTCSessionDescriptionInit }
          | { candidate: RTCIceCandidate };
        if ("candidate" in parsed) {
          console.log("un candidat!!");
          await pc.addIceCandidate(parsed.candidate);
          continue;
        }
        pc.ondatachannel = (event) => {
          console.log("new channel", event.channel.label);
          const receiveChannel = event.channel;
          receiveChannel.binaryType = "arraybuffer";
          setChannel(receiveChannel);
        };
        console.log("offer received");
        await pc.setRemoteDescription(new RTCSessionDescription(parsed.offer));
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        ws.inner.send(JSON.stringify({ answer }));
      }
    })();

    return () => {
      console.log("ras le bol");
      pc.close();
    };
  }, [ws]);

  return channel ? <DataChannelHandler channel={channel} port={port} /> : "rtc wait";
}

interface WsAndMessages {
  inner: WebSocket;
  messages: Messages;
}

function WebRtcMultiplayerOfferer({ port, ws }: { port: MessagePort; ws: WsAndMessages }) {
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
        ws.inner.send(text);
      }
    };

    pc.addEventListener("icecandidate", icecandidateListener);

    const dataChannel = pc.createDataChannel("prout");
    dataChannel.binaryType = "arraybuffer";

    void (async () => {
      while (true) {
        const text = await getTextMessage(ws.messages);
        if (!text) {
          return;
        }
        const parsed = JSON.parse(text) as
          | { answer: RTCSessionDescriptionInit }
          | { candidate: RTCIceCandidate };
        if ("candidate" in parsed) {
          console.log("un candidat!!");
          await pc.addIceCandidate(parsed.candidate);
          continue;
        }
        console.log("answer received");
        await pc.setRemoteDescription(new RTCSessionDescription(parsed.answer));
        setChannel(dataChannel);
      }
    })();

    void pc.createOffer().then(async (offer) => {
      console.log("Offer created:", offer);
      await pc.setLocalDescription(offer);
      ws.inner.send(JSON.stringify({ offer }));
    });

    return () => {
      pc.close();
    };
  }, [ws]);

  return channel ? <DataChannelHandler channel={channel} port={port} /> : "rtc wait";
}

function DataChannelHandler({ channel, port }: { channel: RTCDataChannel; port: MessagePort }) {
  useEffect(() => {
    port.postMessage({
      type: "serialConnected",
    } satisfies FromMainMessage);

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
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    };
  }, [channel, port]);

  return "rtc connected";
}

type Messages = AsyncGenerator<MessageEvent<unknown>>;

async function* websocketGenerator(ws: WebSocket): Messages {
  let queue: MessageEvent<unknown>[] | undefined = [];
  let resolve: undefined | ((value?: MessageEvent<unknown>) => void);

  ws.addEventListener("message", (event) => {
    if (resolve) {
      resolve(event);
      resolve = undefined;
    } else {
      queue?.push(event);
    }
  });

  ws.addEventListener("close", () => {
    if (resolve) {
      resolve();
      resolve = undefined;
    } else {
      queue = undefined;
    }
  });

  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  while (queue) {
    const first = queue.shift();
    if (first) {
      yield first;
      continue;
    }
    const value = await new Promise<MessageEvent<unknown> | undefined>(
      (resolve0) => (resolve = resolve0),
    );
    if (!value) {
      break;
    }
    yield value;
  }
}
