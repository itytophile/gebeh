import { useEffect, useRef } from "react";
import { initCanvas } from ".";

function Canvas({ port }: { port: MessagePort }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current) {
      throw new TypeError("No canvas");
    }
    initCanvas(canvas.current, port);
  }, [port]);

  return <canvas ref={canvas} tabIndex={1} width="160" height="144" />;
}

export default Canvas;
