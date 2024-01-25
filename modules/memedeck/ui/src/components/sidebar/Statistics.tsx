type Props = {
  nMemes: number;
  nCategories: number;
  nTemplates: number;
};

export const Statistics = ({ nMemes, nCategories, nTemplates }: Props) => (
  <div className="flex gap-2">
    <p className="truncate">
      <strong>{nMemes}</strong> memes
    </p>
    <p className="truncate">
      <strong>{nCategories}</strong> categories
    </p>
    <p className="truncate">
      <strong>{nTemplates}</strong> templates
    </p>
  </div>
);
