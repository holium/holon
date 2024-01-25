const walletAddress = "0x1234567890123456789012345678901234567890";

const truncateWalletAddress = (address: string) => {
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const Header = () => (
  <header className="flex justify-between items-center h-16">
    <p>
      ID: <strong>{window.our?.node}</strong>
    </p>
    <a href="/">
      <h1 className="text-3xl font-bold uppercase bangers">Meme Deck</h1>
    </a>
    <p>{truncateWalletAddress(walletAddress)}</p>
  </header>
);
