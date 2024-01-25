import HomeSvg from "../assets/home.svg";
import ChevronSvg from "../assets/chevron.svg";
import SettingsSvg from "../assets/settings.svg";
import OptimismPng from "../assets/optimism.png";
import { Pill } from "./Pill";

const walletAddress = "0x1234567890123456789012345678901234567890";

const truncateWalletAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const Header = () => {
  const truncatedWalletAddress = truncateWalletAddress(walletAddress);
  const nodeParts = window.our?.node.split(".");

  return (
    <header className="flex justify-between items-center h-16">
      <a href="/">
        <Pill>
          <img src={HomeSvg} alt="home" className="w-4 h-4" />
          <p>
            <span className="text-md font-bold opacity-80">{nodeParts[0]}</span>
            <span className="text-md opacity-30">{`.${nodeParts[1]}`}</span>
          </p>
        </Pill>
      </a>
      <a href="/">
        <h1 className="text-3xl font-bold uppercase bangers select-none">
          Meme Deck
        </h1>
      </a>
      <div className="flex items-center gap-3">
        <Pill>
          <img src={OptimismPng} alt="Optimism" className="w-4 h-4" />
          <p className="text-md font-bold opacity-70">
            {truncatedWalletAddress}
          </p>
          <img src={ChevronSvg} alt="Dropdown" className="w-4 h-4" />
        </Pill>
        <Pill>
          <img src={SettingsSvg} alt="Dropdown" className="w-4 h-4" />
        </Pill>
      </div>
    </header>
  );
};
