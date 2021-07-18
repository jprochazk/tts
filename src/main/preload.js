// @ts-check
import { contextBridge, ipcRenderer } from "electron";

/** @type {import("./preload").Api} */
const api = {
  async test() {
    return await ipcRenderer.invoke("api-test");
  },
};
contextBridge.exposeInMainWorld("api", api);
