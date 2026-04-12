import { useCallback, useEffect, useState } from "react";
import type { FromMainMessage, FromNodeMessage } from "../common";
import { getTextMessage, type WsAndMessages } from "./ws-helpers";
import Button from "../bulma/button";
import { RTC_CONFIG } from "../environment";

function useRTCPeerConnection(
  onConnection: (pc: RTCPeerConnection) => void,
  ws: WebSocket,
): RTCIceConnectionState {
  const [iceConnectionState, setIceConnectionState] = useState<RTCIceConnectionState>("new");
  useEffect(() => {
    const pc = new RTCPeerConnection(RTC_CONFIG);

    pc.addEventListener("iceconnectionstatechange", () => {
      setIceConnectionState(pc.iceConnectionState);
    });

    const icecandidateListener = (event: RTCPeerConnectionIceEvent) => {
      if (event.candidate) {
        const text = JSON.stringify({ candidate: event.candidate });
        ws.send(text);
      }
    };

    pc.addEventListener("icecandidate", icecandidateListener);

    onConnection(pc);

    return () => {
      pc.close();
    };
  }, [onConnection, ws]);

  return iceConnectionState;
}

export function WebRtcMultiplayer({ port, ws }: { port: MessagePort; ws: WsAndMessages }) {
  const [channel, setChannel] = useState<RTCDataChannel>();

  const onConnection = useCallback(
    async (pc: RTCPeerConnection) => {
      while (true) {
        const text = await getTextMessage(ws.messages);
        if (!text) {
          return;
        }
        const parsed = JSON.parse(text) as
          | { offer: RTCSessionDescriptionInit }
          | { candidate: RTCIceCandidate };
        if ("candidate" in parsed) {
          console.log("candidate");
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
    },
    [ws],
  );

  const iceConnectionState = useRTCPeerConnection(onConnection, ws.inner);
  return channel ? (
    <DataChannelHandler channel={channel} port={port} />
  ) : (
    <Button
      label={iceConnectionState === "failed" ? "WebRTC failed" : "WebRTC initialization..."}
    />
  );
}

export function WebRtcMultiplayerOfferer({ port, ws }: { port: MessagePort; ws: WsAndMessages }) {
  const [channel, setChannel] = useState<RTCDataChannel>();

  const onConnection = useCallback(
    (pc: RTCPeerConnection) => {
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
            console.log("candidate");
            await pc.addIceCandidate(parsed.candidate);
            continue;
          }
          console.log("answer received");
          await pc.setRemoteDescription(new RTCSessionDescription(parsed.answer));
          setChannel(dataChannel);
        }
      })();

      void pc.createOffer().then(async (offer) => {
        console.log("offer created");
        await pc.setLocalDescription(offer);
        ws.inner.send(JSON.stringify({ offer }));
      });
    },
    [ws],
  );

  const iceConnectionState = useRTCPeerConnection(onConnection, ws.inner);

  return channel ? (
    <DataChannelHandler channel={channel} port={port} />
  ) : (
    <Button
      label={iceConnectionState === "failed" ? "WebRTC failed" : "WebRTC initialization..."}
    />
  );
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
      channel.close();
      port.removeEventListener("message", portListener);
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
    };
  }, [channel, port]);

  return <Button label="Connected 🐣🐔 (WebRTC)" />;
}
