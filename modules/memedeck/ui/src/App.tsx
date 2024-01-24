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
    <div className="px-4 pb-4 max-w-7xl w-full flex flex-col min-h-0">
      <header className="flex justify-between items-center h-16">
        <p>
          ID: <strong>{window.our?.node}</strong>
        </p>
        <a href="/">
          <h1 className="font-bold uppercase">Meme Deck</h1>
        </a>
        <p>{truncateWalletAddress(walletAddress)}</p>
      </header>
      <section className="relative flex justify-between gap-6">
        <aside className="height-full flex flex-col gap-4 min-w-60">
          <div className="flex flex-col gap-2">
            <h3 className="font-bold uppercase">Categories</h3>
            {categories.map((category) => (
              <div key={category}>{category}</div>
            ))}
          </div>
          <div className="flex flex-1 flex-col gap-2">
            <h3 className="font-bold uppercase">Templates</h3>
            {templates.map((template) => (
              <div key={template}>{template}</div>
            ))}
          </div>
          <footer>
            Upload image
          </footer>
        </aside>
        <main className="flex-1 h-full p-5 g-5 rounded-3xl bg-black opacity-35 min-h-80">
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
        <aside className="min-w-60" />
      </section>
    </div>
  );
}

export default App;
