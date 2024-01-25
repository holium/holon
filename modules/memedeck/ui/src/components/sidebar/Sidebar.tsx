import { Statistics } from "./Statistics";
import { PasteLink } from "./PasteLink";

type Props = {
  memes: string[];
  categories: string[];
  templates: string[];
};

export const Sidebar = ({ memes, categories, templates }: Props) => (
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
);
