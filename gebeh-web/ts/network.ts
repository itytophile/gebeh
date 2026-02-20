const roomInput = document.getElementById("roomInput");

if (!(roomInput instanceof HTMLInputElement)) {
  throw new TypeError("roomInput is not an input");
}

const createRoomButton = document.getElementById("createRoomBtn");

if (!(createRoomButton instanceof HTMLButtonElement)) {
  throw new TypeError("createRoomBtn is not an input");
}

const joinRoomButton = document.getElementById("joinRoomBtn");

if (!(joinRoomButton instanceof HTMLButtonElement)) {
  throw new TypeError("joinRoomButton is not an input");
}

createRoomButton.addEventListener("click", () => {
  const ws = new WebSocket("http://localhost:8080");
  ws.addEventListener("open", () => {
    console.log("host!");
  });
  ws.addEventListener("message", (message) => {
    console.log(message.data);
  });
  ws.addEventListener("close", () => {
    console.log("c'est closed");
  });
});

joinRoomButton.addEventListener("click", () => {
  const ws = new WebSocket(`http://localhost:8080?room=${roomInput.value}`);
  ws.addEventListener("open", () => {
    console.log("guest!");
  });
  ws.addEventListener("message", (message) => {
    console.log(message.data);
  });
  ws.addEventListener("close", () => {
    console.log("c'est closed");
  });
});
