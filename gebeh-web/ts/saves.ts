let database: IDBDatabase | undefined;

const OBJECT_STORE_NAME = "saves";

async function getDatabase(): Promise<IDBDatabase> {
  if (database) {
    return database;
  }

  const request = indexedDB.open("gebeh", 1);
  request.onupgradeneeded = () => {
    const database = request.result;

    database.addEventListener("error", (event) => {
      console.error("Error loading database", event);
    });

    database.createObjectStore(OBJECT_STORE_NAME);
  };
  await waitRequest(request);
  database = request.result;
  return database;
}

export async function writeSave(title: string, buffer: ArrayBuffer) {
  const database = await getDatabase();
  const request = database
    .transaction(OBJECT_STORE_NAME, "readwrite")
    .objectStore(OBJECT_STORE_NAME)
    .put(buffer, title);
  await waitRequest(request);
}

export async function getSave(title: string): Promise<ArrayBuffer | undefined> {
  const database = await getDatabase();
  const request = database
    .transaction(OBJECT_STORE_NAME, "readonly")
    .objectStore(OBJECT_STORE_NAME)
    .get(title);
  await waitRequest(request);
  const result: unknown = request.result;

  if (result === undefined || result instanceof ArrayBuffer) {
    return result;
  }

  throw new Error("Unknown object from db for title " + title);
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
