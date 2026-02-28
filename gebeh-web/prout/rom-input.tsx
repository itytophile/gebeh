import type { FromMainMessage } from "./common";
import { getSave } from "./saves";

function getTitleFromRom(rom: Uint8Array): string {
  const title = rom.slice(0x134, 0x143);

  let endZeroPos = title.indexOf(0);
  if (endZeroPos === -1) {
    endZeroPos = title.length;
  }

  const decoder = new TextDecoder("utf-8", { fatal: true });
  return decoder.decode(title.slice(0, endZeroPos));
}

function RomInput({ port }: { port: MessagePort }) {
  return (
    <input
      type="file"
      onChange={async (event) => {
        const file = event.target.files?.item(0);
        if (file) {
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
        } else {
          console.error("Can't load file");
        }
      }}
    />
  );
}

export default RomInput;
