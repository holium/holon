const BASE_URL = import.meta.env.BASE_URL;

if (window.our) window.our.process = BASE_URL?.replace("/", "");

export const PROXY_TARGET = `${import.meta.env.VITE_NODE_URL || "http://localhost:8080"}${BASE_URL}`;
