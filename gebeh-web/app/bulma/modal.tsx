import type { PropsWithChildren } from "react";

export function Modal({ isActive, children }: PropsWithChildren<{ isActive: boolean }>) {
  return (
    <div className={"modal" + (isActive ? " is-active" : "")}>
      <div className="modal-background" />
      <div className="modal-card">{children}</div>
    </div>
  );
}

export function ModalBody({ children }: PropsWithChildren) {
  return <section className="modal-card-body">{children}</section>;
}

export function ModalFooter({ children }: PropsWithChildren) {
  return <footer className="modal-card-foot">{children}</footer>;
}
