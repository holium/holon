import { useState, useEffect } from "react";

const BASE_URL = import.meta.env.BASE_URL;
if (window.our) window.our.process = BASE_URL?.replace("/", "");

const PROXY_TARGET = `${(import.meta.env.VITE_NODE_URL || "http://localhost:8080")}${BASE_URL}`;

const walletAddress = "0x1234567890123456789012345678901234567890";

const truncateWalletAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

const categories = ["shizo", "epstein", "e/acc", "decels", "trump"]
const templates = ["bell curve", "distracted boyfriend", "expanding brain", "anakin padme 4 panel", "two buttons"]

function App() {
  const [nodeConnected, setNodeConnected] = useState(false);

  useEffect(() => {
    if (window.our?.node && window.our?.process) {
      setNodeConnected(true);
    } else {
      setNodeConnected(false);
    }
  }, []);

  return (
    <div className="px-4 max-w-5xl w-full">
      <header className="flex justify-between items-center h-14">
        <p>
          ID: <strong>{window.our?.node}</strong>
        </p>
        <h1>Meme Deck</h1>
        <p>{truncateWalletAddress(walletAddress)}</p>
      </header>
      <main className="flex">
        <aside>
          <h3>Categories</h3>
          {categories.map((category) => (
            <div key={category}>{category}</div>
          ))}
          <h3>Templates</h3>
          {templates.map((template) => (
            <div key={template}>{template}</div>
          ))}
        </aside>
        {!nodeConnected && (
          <div className="node-not-connected">
            <h2 style={{ color: "red" }}>Node not connected</h2>
            <h4>
              You need to start a node at {PROXY_TARGET} before you can use this UI
              in development.
            </h4>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
