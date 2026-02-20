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

const roomDiv = document.getElementById("room");

if (!(roomDiv instanceof HTMLDivElement)) {
  throw new TypeError("roomDiv is not a div");
}

const CLOSE_MESSAGE = "Room closed 🍗🍗";

function getReadyRoomMessage(room: string) {
  return `${room} 🐣🐔`;
}

createRoomButton.addEventListener("click", () => {
  const ws = new WebSocket("http://localhost:8080");
  ws.addEventListener("open", () => {
    console.log("host!");
  });

  let state:
    | { type: "waitName" }
    | { type: "waitGuest"; room: string }
    | { type: "done" } = {
    type: "waitName",
  };

  ws.addEventListener("message", (message) => {
    switch (state.type) {
      case "waitName": {
        if (typeof message.data !== "string") {
          throw new TypeError("First message must be the room name");
        }
        roomDiv.textContent = `${message.data} 🥚🐔`;
        state = { type: "waitGuest", room: message.data };
        break;
      }
      case "waitGuest": {
        roomDiv.textContent = getReadyRoomMessage(state.room);
        state = { type: "done" };
        break;
      }
    }
    console.log(message.data);
  });
  ws.addEventListener("close", () => {
    roomDiv.textContent = CLOSE_MESSAGE;
  });
});

joinRoomButton.addEventListener("click", () => {
  const room = roomInput.value.trim();
  const ws = new WebSocket(`http://localhost:8080?room=${room}`);
  ws.addEventListener("open", () => {
    roomDiv.textContent = getReadyRoomMessage(room);
    console.log("guest!");
  });
  ws.addEventListener("message", (message) => {
    console.log(message.data);
  });
  ws.addEventListener("close", () => {
    roomDiv.textContent = CLOSE_MESSAGE;
  });
});
