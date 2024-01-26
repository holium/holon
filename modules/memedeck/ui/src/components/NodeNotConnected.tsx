import { PROXY_TARGET } from "../util/proxy";

export const NodeNotConnected = () => (
  <div>
    <h2 className="text-red-500">Node not connected</h2>
    <p>
      You need to start a node at {PROXY_TARGET} before you can use this UI in
      development.
    </p>
  </div>
);
