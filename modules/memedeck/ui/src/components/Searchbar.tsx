import { useRef } from "react";
import ChevronSvg from "../assets/chevron.svg";
import PostsSvg from "../assets/posts.svg";
import SearchSvg from "../assets/search.svg";

type Props = {
  onSearch: (query: string) => void;
};

export const SearchBar = ({ onSearch }: Props) => {
  const inputRef = useRef<HTMLInputElement>(null);

  const debounce = (func: (...args: unknown[]) => void, wait: number) => {
    let timeout: NodeJS.Timeout;
    return function executedFunction(...args: unknown[]) {
      const later = () => {
        clearTimeout(timeout);
        func(...args);
      };
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
    };
  };

  const debouncedSearch = debounce(onSearch, 500);

  return (
    <div className="flex gap-4 relative w-full">
      <img
        src={SearchSvg}
        alt="Dropdown"
        onClick={(e) => {
          e.preventDefault();
          inputRef.current.focus();
        }}
        className="w-5 h-5 absolute left-3 top-2.5 opacity-75 user-select-none cursor-text"
      />
      <input
        ref={inputRef}
        placeholder="Search for memes. (i.e. bell curve)"
        className="flex-1 pl-10 pr-4 bg-black-32 h-10 rounded-full placeholder-white-32 border border-white-4"
        onChange={(e) => debouncedSearch(e.target.value)}
      />
      <div className="flex gap-1 items-center cursor-pointer">
        <p className="text-sm uppercase opacity-70">Random</p>
        <img src={ChevronSvg} alt="Dropdown" className="w-4 h-4" />
      </div>
      <div className="flex gap-1 items-center cursor-pointer">
        <img src={PostsSvg} alt="Dropdown" className="w-6 h-6" />
        <img src={ChevronSvg} alt="Dropdown" className="w-4 h-4" />
      </div>
    </div>
  );
};
