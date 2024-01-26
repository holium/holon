type Props = {
  nMemes: number;
  nCategories: number;
  nTemplates: number;
};

const Stat = ({ value, label }: { value: number; label: string }) => (
  <p className="truncate text-sm">
    <span className="font-bold">{value}</span>
    <span className="opacity-60"> {label}</span>
  </p>
);

export const Statistics = ({ nMemes, nCategories, nTemplates }: Props) => (
  <div className="flex gap-2">
    <Stat value={nMemes} label="memes" />
    <Stat value={nCategories} label="categories" />
    <Stat value={nTemplates} label="templates" />
  </div>
);
