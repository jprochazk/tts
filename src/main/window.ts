import {
  BrowserWindow,
  BrowserWindowConstructorOptions,
  WebPreferences,
} from "electron";
import { getAssetURL } from "electron-snowpack";
import path from "path";

const requiredWebPreferences: WebPreferences = {
  nodeIntegration: false,
  contextIsolation: true,
  enableRemoteModule: false,
  preload: path.join(__dirname, "preload.js"),
  devTools: process.env.MODE === "development",
} as const;

export type WindowOptions = Omit<
  BrowserWindowConstructorOptions,
  "webPreferences"
> & {
  menuBarVisible?: boolean;
  webPreferences?: Omit<WebPreferences, keyof typeof requiredWebPreferences>;
};

export class Window extends BrowserWindow {
  constructor(file: string, options?: WindowOptions) {
    super({
      ...options,
      webPreferences: {
        ...options?.webPreferences,
        ...requiredWebPreferences,
      },
    });
    this.menuBarVisible = options?.menuBarVisible ?? false;
    this.loadURL(getAssetURL(file));
  }
}
