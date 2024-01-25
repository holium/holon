import { useState } from "react";

export const PasteLink = () => {
  const [uploadLink, setUploadLink] = useState("");

  const onClickUpload = () => {
    const valid = uploadLink.startsWith("http");
    if (!valid) {
      alert("Invalid link");
      return;
    }
    console.log("Uploading", uploadLink);
  };

  return (
    <div className="flex gap-2">
      <input
        placeholder="Paste link"
        value={uploadLink}
        onChange={(e) => setUploadLink(e.target.value)}
        type="text"
        className="flex-1 text-sm h-8 px-3 pr-4 bg-black-32 rounded-xl placeholder-white-32 border border-white-4"
      />
      <button
        onClick={onClickUpload}
        disabled={!uploadLink}
        className="bg-blue-500 text-sm h-8 px-3 rounded-xl cursor-pointer"
      >
        Upload
      </button>
    </div>
  );
};
