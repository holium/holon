import { useState, useEffect } from "react";
import { NodeNotConnected } from "./components/NodeNotConnected";
import { Header } from "./components/Header";
import { Sidebar } from "./components/sidebar/Sidebar";
import { categories, memes, templates } from "./util/data";
import { SearchBar } from "./components/SearchBar";
import { BASE_URL } from "./util/proxy";

function App() {
  const [nodeConnected, setNodeConnected] = useState(false);

  useEffect(() => {
    // Get message history using http
    fetch(`${BASE_URL}/messages`)
      .then((response) => response.json())
      .then((data) => {
        console.log(data);
      })
      .catch((error) => console.error(error));

    if (window.our?.node && window.our?.process) {
      setNodeConnected(true);
    } else {
      setNodeConnected(false);
    }
  }, []);

  return (
    <div className="px-4 pb-4 gap-2 max-w-7xl w-full flex flex-col min-h-0">
      <Header />
      <section className="flex flex-1 min-h-0 justify-between gap-6">
        <Sidebar memes={memes} categories={categories} templates={templates} />
        <main className="flex flex-col flex-1 h-full gap-3">
          <SearchBar />
          <div className="flex flex-col flex-1 h-full p-5 gap-5 overflow-y-scroll rounded-3xl bg-black-200 border border-white-4">
            {!nodeConnected ? (
              <NodeNotConnected />
            ) : (
              <>
                {memes.map((meme) => (
                  <a href={`#${meme}`} key={meme}>
                    <img
                      src={meme}
                      alt="meme"
                      className="rounded-xl w-full h-auto"
                    />
                  </a>
                ))}
              </>
            )}
          </div>
        </main>
        {/* Hide on tablet/phone */}
        <aside className="min-w-60 hidden xl:flex" />
      </section>
    </div>
  );
}

export default App;
