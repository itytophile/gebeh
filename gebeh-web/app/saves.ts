const OBJECT_STORE_NAME = "saves";

const DATABASE = new Promise<IDBDatabase>((resolve) => {
  const request = indexedDB.open("gebeh", 1);
  request.onupgradeneeded = () => {
    const database = request.result;

    database.addEventListener("error", (event) => {
      console.error("Error loading database", event);
    });

    database.createObjectStore(OBJECT_STORE_NAME);
  };
  void waitRequest(request).then(() => {
    resolve(request.result);
  });
});

export async function writeSave(title: string, buffer: Uint8Array) {
  const database = await DATABASE;
  const request = database
    .transaction(OBJECT_STORE_NAME, "readwrite")
    .objectStore(OBJECT_STORE_NAME)
    .put(buffer, title);
  await waitRequest(request);
}

export function writeExtra(title: string, buffer: Uint8Array) {
  // yeah that's called a hack
  return writeSave(title + "_EXTRA", buffer);
}

export async function deleteSave(title: string) {
  const database = await DATABASE;
  const request = database
    .transaction(OBJECT_STORE_NAME, "readwrite")
    .objectStore(OBJECT_STORE_NAME)
    .delete(title);
  await waitRequest(request);
}

export async function getKeys(): Promise<IDBValidKey[]> {
  const database = await DATABASE;
  const request = database
    .transaction(OBJECT_STORE_NAME, "readonly")
    .objectStore(OBJECT_STORE_NAME)
    .getAllKeys();
  await waitRequest(request);
  return request.result;
}

export async function getSave(title: string): Promise<Uint8Array | undefined> {
  const database = await DATABASE;
  const request = database
    .transaction(OBJECT_STORE_NAME, "readonly")
    .objectStore(OBJECT_STORE_NAME)
    .get(title);
  await waitRequest(request);
  const result: unknown = request.result;

  if (result === undefined) {
    return undefined;
  }

  if (result instanceof Uint8Array && result.buffer instanceof ArrayBuffer) {
    return new Uint8Array(result.buffer);
  }

  throw new Error("Unknown object from db for title " + title);
}

export function getExtra(title: string): Promise<Uint8Array | undefined> {
  return getSave(title + "_EXTRA");
}

function waitRequest(request: IDBRequest): Promise<void> {
  return new Promise((resolve, reject) => {
    request.addEventListener("success", () => {
      resolve();
    });
    request.addEventListener("error", () => {
      reject(request.error ?? new TypeError("Null error"));
    });
  });
}
