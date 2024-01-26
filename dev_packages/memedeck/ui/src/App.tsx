import { useState, useEffect } from "react";
import { NodeNotConnected } from "./components/NodeNotConnected";
import { Header } from "./components/Header";
import { Sidebar } from "./components/sidebar/Sidebar";
import { SearchBar } from "./components/Searchbar";
import { BASE_URL } from "./util/proxy";
import { Meme, MemeCategory, MemeTemplate } from "./util/types";

function App() {
  const [nodeConnected, setNodeConnected] = useState(false);
  const [memes, setMemes] = useState<Meme[]>([]);
  const [filteredMemes, setFilteredMemes] = useState<Meme[]>([]);
  const [categories, setCategories] = useState<MemeCategory[]>([]);
  const [templates, setTemplates] = useState<MemeTemplate[]>([]);

  useEffect(() => {
    // Get message history using http
    fetch(`${BASE_URL}/categories`)
      .then((response) => response.json())
      .then((data) => {
        setCategories(data);
      })
      .catch((error) => console.error(error));

    // Get templates
    fetch(`${BASE_URL}/templates`)
      .then((response) => response.json())
      .then((data) => {
        setTemplates(data);
      })
      .catch((error) => console.error(error));

    // Get memes
    fetch(`${BASE_URL}/memes`)
      .then((response) => response.json())
      .then((data) => {
        setMemes(data);
        setFilteredMemes(data);
      })
      .catch((error) => console.error(error));

    if (window.our?.node && window.our?.process) {
      setNodeConnected(true);
    } else {
      setNodeConnected(false);
    }
  }, []);

  const onSearch = (query: string) => {
    if (!query) {
      setFilteredMemes(memes);
      return;
    }

    fetch(`${BASE_URL}/memes?query=${query}`)
      .then((response) => response.json())
      .then((data) => {
        setFilteredMemes(data);
      })
      .catch((error) => console.error(error));
  };

  return (
    <div className="px-4 pb-4 gap-2 max-w-7xl w-full flex flex-col min-h-0">
      <Header />
      <section className="flex flex-1 min-h-0 justify-between gap-6">
        <Sidebar memes={memes} categories={categories} templates={templates} />
        <main className="flex flex-col flex-1 h-full gap-3">
          <SearchBar onSearch={onSearch} />
          <div className="flex flex-col flex-1 h-full p-5 gap-5 overflow-y-scroll rounded-3xl bg-black-200 border border-white-4">
            {!nodeConnected ? (
              <NodeNotConnected />
            ) : (
              <>
                {filteredMemes
                  .sort((a, b) => (a.id < b.id ? 1 : -1))
                  .map((meme) => (
                    <a href={`#${meme.id}`} key={meme.id}>
                      <img
                        src={BASE_URL + meme.url}
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
