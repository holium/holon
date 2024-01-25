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
        className="bg-black-200 h-8 px-3 rounded-xl"
      />
      <button
        onClick={onClickUpload}
        disabled={!uploadLink}
        className="bg-blue-400 h-8 px-3 rounded-xl"
      >
        Upload
      </button>
    </div>
  );
};
