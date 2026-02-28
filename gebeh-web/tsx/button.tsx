import type { CSSProperties } from "react";
import type { GebehButton, FromMainMessage } from "./common";

function Button({
  src,
  port,
  button,
  style,
}: {
  src: string;
  port: MessagePort;
  button: GebehButton;
  style?: CSSProperties;
}) {
  return (
    <img
      className="interactive"
      style={style}
      src={src}
      onPointerDown={(event) => {
        event.preventDefault();
        port.postMessage({
          type: "input",
          event: "down",
          button,
        } satisfies FromMainMessage);
      }}
      onPointerUp={() => {
        port.postMessage({
          type: "input",
          event: "up",
          button,
        } satisfies FromMainMessage);
      }}
    />
  );
}

export default Button;
