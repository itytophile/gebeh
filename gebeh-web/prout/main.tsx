import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./app";

const root = document.getElementById("root");

if (!root) {
  throw new TypeError("Can't find element with id 'root'");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
