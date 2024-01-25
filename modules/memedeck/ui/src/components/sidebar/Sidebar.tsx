import { Statistics } from "./Statistics";
import { PasteLink } from "./PasteLink";
import { Category, MemeTemplate } from "../../util/data";

type Props = {
  memes: string[];
  categories: Category[];
  templates: MemeTemplate[];
};

export const Sidebar = ({ memes, categories, templates }: Props) => {
  const nMemes = memes.length;
  const nCategories = categories.reduce(
    (acc, category) => acc + category.count,
    0,
  );
  const nTemplates = templates.reduce(
    (acc, template) => acc + template.count,
    0,
  );

  return (
    <aside className="flex-col gap-6 min-w-60 hidden md:flex">
      <div className="flex flex-col gap-2">
        <h3 className="font-bold text-sm uppercase">Categories</h3>
        {categories.map((category) => (
          <a key={category.name} href={`#${category}`} className="flex">
            <div className="flex-1 opacity-70">{category.name}</div>
            <div className="opacity-40">{category.count}</div>
          </a>
        ))}
      </div>
      <div className="flex flex-1 flex-col gap-2">
        <h3 className="font-bold text-sm uppercase">Templates</h3>
        {templates.map((template) => (
          <a key={template.name} href={`#${template}`} className="flex">
            <div className="flex-1 opacity-70">{template.name}</div>
            <div className="opacity-40">{template.count}</div>
          </a>
        ))}
      </div>
      <footer className="flex flex-col gap-4">
        <Statistics
          nMemes={nMemes}
          nCategories={nCategories}
          nTemplates={nTemplates}
        />
        <PasteLink />
      </footer>
    </aside>
  );
};
