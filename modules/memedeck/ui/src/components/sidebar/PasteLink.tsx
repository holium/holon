import { useState } from "react";
import { BASE_URL } from "../../util/proxy";

export const PasteLink = () => {
  const [uploading, setUploading] = useState(false);
  const [uploadLink, setUploadLink] = useState("");

  const onClickUpload = () => {
    const valid = uploadLink.startsWith("http");
    if (!valid) {
      alert("Invalid link");
      return;
    }

    setUploading(true);
    console.log("Uploading", uploadLink);

    fetch(`${BASE_URL}/upload`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ url: uploadLink }),
    })
      .then((response) => response.json())
      .then((data) => {
        console.log("Upload", data);
      })
      .catch((error) => console.error(error))
      .finally(() => {
        setUploading(false);
        setUploadLink("");
      });
  };

  return (
    <div className="flex gap-2">
      <input
        placeholder="Paste link"
        value={uploadLink}
        type="text"
        className="flex-1 text-sm h-8 px-3 pr-4 bg-black-32 rounded-xl placeholder-white-32 border border-white-4"
        onChange={(e) => setUploadLink(e.target.value)}
      />
      <button
        disabled={!uploadLink || uploading}
        className="bg-blue-500 text-sm h-8 px-3 rounded-xl cursor-pointer"
        onClick={onClickUpload}
      >
        Upload
      </button>
    </div>
  );
};
