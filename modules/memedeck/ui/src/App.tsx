import { useState, useEffect } from "react";
import MockMeme from "./assets/cat.png";
import { NodeNotConnected } from "./components/NodeNotConnected";
import { Header } from "./components/Header";
import { Statistics } from "./components/Statistics";
import { PasteLink } from "./components/PasteLink";

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

  const [memes, setMemes] = useState<string[]>([]);

  useEffect(() => {
    setMemes([MockMeme, MockMeme, MockMeme, MockMeme, MockMeme]);

    if (window.our?.node && window.our?.process) {
      setNodeConnected(true);
    } else {
      setNodeConnected(false);
    }
  }, []);

  return (
    <div className="px-4 pb-4 max-w-7xl w-full flex flex-col min-h-0">
      <Header />
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
            <Statistics
              nMemes={memes.length}
              nCategories={categories.length}
              nTemplates={templates.length}
            />
            <PasteLink />
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
