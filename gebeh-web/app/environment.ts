declare global {
  var __GEBEH_ENV__:
    | {
        stunServer?: unknown;
      }
    | undefined;
}

if (typeof globalThis.__GEBEH_ENV__?.stunServer != "string") {
  throw new TypeError("Please provide stunServer (string) in __GEBEH_ENV__");
}

console.log("STUN server:", globalThis.__GEBEH_ENV__.stunServer);

export const RTC_CONFIG = {
  iceServers: [
    {
      urls: globalThis.__GEBEH_ENV__.stunServer,
    },
  ],
};
