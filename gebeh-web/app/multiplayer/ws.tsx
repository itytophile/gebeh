import { useEffect } from "react";
import type { FromMainMessage, FromNodeMessage } from "../common";
import type { WsAndMessages } from "./ws-helpers";
import Button from "../bulma/button";

function WebSocketMultiplayer({ port, ws }: { port: MessagePort; ws: WsAndMessages }) {
  useEffect(() => {
    const portListener = ({ data }: MessageEvent<FromNodeMessage>) => {
      if (data.type === "serial") {
        ws.inner.send(data.buffer);
      }
    };

    void (async () => {
      for await (const message of ws.messages) {
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
      }
    })();

    port.addEventListener("message", portListener);

    port.postMessage({
      type: "serialConnected",
    } satisfies FromMainMessage);

    return () => {
      port.postMessage({
        type: "serialDisconnected",
      } satisfies FromMainMessage);
      port.removeEventListener("message", portListener);
    };
  }, [ws, port]);

  return <Button label="Connected 🐣🐔" />;
}

export default WebSocketMultiplayer;
