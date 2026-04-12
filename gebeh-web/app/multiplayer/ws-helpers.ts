export async function getBinaryMessage(
  messages: AsyncGenerator<MessageEvent<unknown>>,
): Promise<ArrayBuffer | undefined> {
  const result = await messages.next();
  if (result.done) {
    return undefined;
  }
  if (!(result.value.data instanceof ArrayBuffer)) {
    throw new TypeError("First message must be the room name");
  }
  return result.value.data;
}

export async function getTextMessage(
  messages: AsyncGenerator<MessageEvent<unknown>>,
): Promise<string | undefined> {
  const result = await messages.next();
  if (result.done) {
    return undefined;
  }
  if (typeof result.value.data !== "string") {
    throw new TypeError("First message must be the room name");
  }
  return result.value.data;
}

export interface WsAndMessages {
  inner: WebSocket;
  // I need to buffer incoming messages because I can miss them when subcomponent begins to read
  // the messages
  messages: Messages;
}

export type Messages = AsyncGenerator<MessageEvent<unknown>>;

export async function* websocketGenerator(ws: WebSocket): Messages {
  const queue: ({ value: MessageEvent<unknown>; done: false } | { done: true })[] = [];
  let resolve: undefined | ((value?: MessageEvent<unknown>) => void);

  ws.addEventListener("message", (event) => {
    if (resolve) {
      resolve(event);
      resolve = undefined;
    } else {
      queue.push({ done: false, value: event });
    }
  });

  ws.addEventListener("close", () => {
    if (resolve) {
      resolve();
      resolve = undefined;
    } else {
      queue.push({ done: true });
    }
  });

  while (true) {
    const first = queue.shift();
    if (first) {
      if (first.done) {
        return;
      }
      yield first.value;
      continue;
    }
    const value = await new Promise<MessageEvent<unknown> | undefined>(
      (resolve0) => (resolve = resolve0),
    );
    if (!value) {
      break;
    }
    yield value;
  }
}
