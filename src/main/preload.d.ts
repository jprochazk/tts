declare global {
  interface Window {
    api: Api;
  }
}

export type Api = {
  test(): Promise<string>;
};
