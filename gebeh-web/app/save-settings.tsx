import { faDownload, faXmark } from "@fortawesome/free-solid-svg-icons";
import { useState, useEffect } from "react";
import Button from "./bulma/button";
import Select from "./bulma/select";
import { deleteSave, getKeys, getSave, writeSave } from "./saves";
import FileInput from "./bulma/file-input";
import { faUpload } from "@fortawesome/free-solid-svg-icons/faUpload";
import { Modal, ModalBody, ModalFooter } from "./bulma/modal";

function SaveSettings() {
  const [databaseKeys, setDatabaseKeys] = useState<IDBValidKey[]>();
  const [selectedSave, setSelectedSave] = useState<string>();
  const [resolveConfirm, setResolveConfirm] = useState<(confirm: boolean) => void>();

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
        <Button
          icon={faXmark}
          label="Clear save"
          onClick={
            selectedSave
              ? async () => {
                  // cursed but who cares, trying things
                  const confirm = await new Promise<boolean>((resolve) => {
                    setResolveConfirm(resolve);
                  });
                console.log({ confirm });
                  if (confirm) {
                    console.log("on est ionb");
                    await deleteSave(selectedSave);
                  } else {
                    console.log("canceled lol");
                  }
                }
              : undefined
          }
          disabled={!selectedSave}
        />
      </div>
      {resolveConfirm && (
        <Modal isActive>
          <ModalBody>Are you sure?</ModalBody>
          <ModalFooter>
            <div className="buttons">
              <Button
                label="Cancel"
                color="is-primary"
                onClick={() => {
                  resolveConfirm(false);
                  setResolveConfirm(undefined);
                }}
              />
              <Button
                label="Confirm"
                color="is-danger"
                onClick={() => {
                  resolveConfirm(true);
                  setResolveConfirm(undefined);
                }}
              />
            </div>
          </ModalFooter>
        </Modal>
      )}
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
