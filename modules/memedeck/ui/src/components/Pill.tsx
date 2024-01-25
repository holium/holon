type Props = {
  children: React.ReactNode;
  className?: string;
};

export const Pill = ({ children, className }: Props) => (
  <div
    className={`pointer flex items-center gap-2 px-3 rounded-full h-9 bg-white-4 border-white-10 border text-white ${className}`}
  >
    {children}
  </div>
);
