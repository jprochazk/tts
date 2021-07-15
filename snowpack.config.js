/** @type { import("snowpack").SnowpackUserConfig } */
module.exports = {
  extends: "electron-snowpack/config/snowpack.js",
  plugins: ["@snowpack/plugin-svelte", "@snowpack/plugin-postcss"],
  devOptions: {
    tailwindConfig: "./tailwind.config.js",
  },
};
