import { app, ipcMain } from "electron";
import { Window } from "./window";

let mainWindow: Window | null = null;

app.on("window-all-closed", (): void => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", (): void => {
  mainWindow ??= new Window("index.html");
  mainWindow.on("closed", () => (mainWindow = null));
});

app.on("ready", (): void => {
  mainWindow ??= new Window("index.html");
  mainWindow.on("closed", () => (mainWindow = null));
});

ipcMain.handle("api-test", () => {
  return "Hello!";
});
