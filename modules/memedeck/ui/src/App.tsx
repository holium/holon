import { useState, useEffect } from "react";
import MockMeme from "./assets/cat.png";
import { NodeNotConnected } from "./components/NodeNotConnected";

const walletAddress = "0x1234567890123456789012345678901234567890";

const truncateWalletAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

const categories = ["shizo", "epstein", "e/acc", "decels", "trump"];
const templates = [
  "bell curve",
  "distracted boyfriend",
  "expanding brain",
  "anakin padme 4 panel",
  "two buttons",
];

function App() {
  const [nodeConnected, setNodeConnected] = useState(false);

  const [memes, setMemes] = useState([MockMeme]);
  const [uploadLink, setUploadLink] = useState("");

  const onClickUpload = () => {
    const valid = uploadLink.startsWith("http");
    if (!valid) {
      alert("Invalid link");
      return;
    }
    console.log("Uploading", uploadLink);
  };

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
          <h1 className="text-3xl font-bold uppercase bangers">Meme Deck</h1>
        </a>
        <p>{truncateWalletAddress(walletAddress)}</p>
      </header>
      <section className="h-full flex justify-between gap-6">
        <aside className="flex-col gap-6 min-w-60 hidden md:flex">
          <div className="flex flex-col gap-2">
            <h3 className="font-bold uppercase">Categories</h3>
            {categories.map((category) => (
              <a key={category} href={`#${category}`}>
                <div>{category}</div>
              </a>
            ))}
          </div>
          <div className="flex flex-1 flex-col gap-2">
            <h3 className="font-bold uppercase">Templates</h3>
            {templates.map((template) => (
              <a key={template} href={`#${template}`}>
                <div>{template}</div>
              </a>
            ))}
          </div>
          <footer className="flex flex-col gap-4">
            <div className="flex gap-2">
              <p className="truncate">
                <strong>1237</strong> memes
              </p>
              <p className="truncate">
                <strong>{categories.length}</strong> categories
              </p>
              <p className="truncate">
                <strong>{templates.length}</strong> templates
              </p>
            </div>
            <div className="flex gap-2">
              <input
                placeholder="Paste link"
                value={uploadLink}
                onChange={(e) => setUploadLink(e.target.value)}
                type="text"
                className="bg-black-200 h-8 px-3 rounded-xl"
              />
              <button
                onClick={onClickUpload}
                disabled={!uploadLink}
                className="bg-blue-400 h-8 px-3 rounded-xl"
              >
                Upload
              </button>
            </div>
          </footer>
        </aside>
        <main className="flex-1 h-full p-5 g-5 rounded-3xl bg-black-200 min-h-80">
          {!nodeConnected ? (
            <NodeNotConnected />
          ) : (
            <>
              {memes.map((meme) => (
                <img
                  key={meme}
                  src={meme}
                  alt="meme"
                  className="rounded-xl w-full"
                />
              ))}
            </>
          )}
        </main>
        {/* Hide on tablet/phone */}
        <aside className="min-w-60 hidden xl:flex" />
      </section>
    </div>
  );
}

export default App;
