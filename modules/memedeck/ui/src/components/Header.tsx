import HomeSvg from "../assets/home.svg";

const walletAddress = "0x1234567890123456789012345678901234567890";

const truncateWalletAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const Header = () => {
  const truncatedWalletAddress = truncateWalletAddress(walletAddress);
  const nodeParts = window.our?.node.split(".");

  return (
    <header className="flex justify-between items-center h-16">
      <a
        href="/"
        className="pointer flex items-center gap-2 px-3 rounded-full h-9 bg-white-4 border-white-10 border text-white"
      >
        <img src={HomeSvg} alt="home" className="w-4 h-4" />
        <p>
          <span className="text-md font-bold opacity-80">{nodeParts[0]}</span>
          <span className="text-md opacity-30">{`.${nodeParts[1]}`}</span>
        </p>
      </a>
      <a href="/">
        <h1 className="text-3xl font-bold uppercase bangers">Meme Deck</h1>
      </a>
      <p>{truncatedWalletAddress}</p>
    </header>
  );
};
