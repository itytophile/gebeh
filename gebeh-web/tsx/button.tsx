import type { CSSProperties } from "react";
import type { GebehButton, FromMainMessage } from "./common";
import style from "../style.module.css";

function Button({
  src,
  port,
  button,
  style: styleProperty,
}: {
  src: string;
  port: MessagePort;
  button: GebehButton;
  style?: CSSProperties;
}) {
  return (
    <img
      className={style.interactive}
      style={styleProperty}
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
