import { faDownload, faXmark } from "@fortawesome/free-solid-svg-icons";
import { useState, useEffect } from "react";
import Button from "./bulma/button";
import Select from "./bulma/select";
import { getKeys, getSave, writeSave } from "./saves";
import FileInput from "./bulma/file-input";
import { faUpload } from "@fortawesome/free-solid-svg-icons/faUpload";

function SaveSettings() {
  const [databaseKeys, setDatabaseKeys] = useState<IDBValidKey[]>();
  const [selectedSave, setSelectedSave] = useState<string>();

  useEffect(() => {
    void getKeys().then(setDatabaseKeys);
  }, []);

  const handleFileLoad: React.ChangeEventHandler<HTMLInputElement> | undefined = selectedSave
    ? async (event) => {
        const file = event.currentTarget.files?.item(0);
        if (file) {
          await writeSave(selectedSave, await file.bytes());
        }
      }
    : undefined;

  return (
    <>
      <div className="field">
        <Select
          options={(databaseKeys ?? [])
            .filter((key) => typeof key === "string")
            .map((key) => ({ label: key, value: key }))}
          onClick={(option) => {
            setSelectedSave(option?.value);
          }}
          selected={selectedSave}
        />
      </div>

      <div className="field">
        <Button
          icon={faDownload}
          label="Download save"
          onClick={
            selectedSave
              ? async () => {
                  const save = await getSave(selectedSave);
                  if (save) {
                    downloadFile(save, selectedSave);
                  }
                }
              : undefined
          }
          disabled={!selectedSave}
        />
      </div>
      <div className="field">
        {handleFileLoad ? (
          <FileInput label="Load save" onChange={handleFileLoad} />
        ) : (
          <Button label="Load save" icon={faUpload} disabled />
        )}
      </div>
      <div className="field">
        <Button icon={faXmark} label="Clear save" />
      </div>
    </>
  );
}

export default SaveSettings;

function downloadFile(bytes: Uint8Array<ArrayBuffer>, fileName: string) {
  const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = `${fileName}.bin`;
  document.body.append(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
