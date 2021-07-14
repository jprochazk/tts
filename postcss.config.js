const tailwind = require('tailwindcss');
const cssnano = require('cssnano');

const plugins = process.env.NODE_ENV === 'production'
    ? [tailwind, cssnano]
    : [tailwind]

module.exports = { plugins };