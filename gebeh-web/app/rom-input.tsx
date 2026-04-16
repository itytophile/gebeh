import type { FromMainMessage } from "./common";
import { getSave } from "./saves";
import { useState } from "react";
import FileInput from "./bulma/file-input";

function getTitleFromRom(rom: Uint8Array): string {
  const title = rom.slice(0x134, 0x143);

  let endZeroPos = title.indexOf(0);
  if (endZeroPos === -1) {
    endZeroPos = title.length;
  }

  const decoder = new TextDecoder("utf-8", { fatal: true });
  return decoder.decode(title.slice(0, endZeroPos));
}

function RomInput({ port, onLoad }: { port: MessagePort; onLoad?: () => void }) {
  const [fileName, setFileName] = useState<string>();
  const onFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.item(0);
    if (file) {
      setFileName(file.name);
      const bytes = new Uint8Array(await file.arrayBuffer());

      const save = await getSave(getTitleFromRom(new Uint8Array(bytes)));
      port.postMessage(
        {
          type: "rom",
          bytes,
          save,
          seconds_since_epoch: Date.now() / 1000,
        } satisfies FromMainMessage,
        save ? [bytes.buffer, save.buffer] : [bytes.buffer],
      );
      onLoad?.();
    } else {
      console.error("Can't load file");
    }
  };
  return (
    <div className="field">
      <FileInput label="Load ROM" fileName={fileName} onChange={onFileChange} color="is-success" />
    </div>
  );
}

export default RomInput;
