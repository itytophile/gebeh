import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import type { FromMainMessage } from "./common";
import { getSave } from "./saves";
import { faUpload } from "@fortawesome/free-solid-svg-icons/faUpload";
import { useState } from "react";

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
      <div className={"file" + (fileName ? " has-name" : "")}>
        <label className="file-label">
          <input className="file-input" type="file" onChange={onFileChange} />
          <span className="file-cta">
            <span className="file-icon">
              <FontAwesomeIcon icon={faUpload} />
            </span>
            <span className="file-label">Load ROM</span>
          </span>
          {fileName && <span className="file-name">{fileName}</span>}
        </label>
      </div>
    </div>
  );
}

export default RomInput;
