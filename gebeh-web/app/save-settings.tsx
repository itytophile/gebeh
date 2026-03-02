import { faDownload, faUpload, faXmark } from "@fortawesome/free-solid-svg-icons";
import { useState, useEffect } from "react";
import Button from "./bulma/button";
import Select from "./bulma/select";
import { getKeys, getSave } from "./saves";

function SaveSettings() {
  const [databaseKeys, setDatabaseKeys] = useState<IDBValidKey[]>();

  useEffect(() => {
    void getKeys().then(setDatabaseKeys);
  }, []);

  const [selectedSave, setSelectedSave] = useState<string>();
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
        <Button icon={faUpload} label="Load save" />
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
