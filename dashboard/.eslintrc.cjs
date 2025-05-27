module.exports = {
  parser: '@typescript-eslint/parser',
  plugins: ['@typescript-eslint', 'react'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react/recommended',
    'prettier'
  ],
  rules: {
    quotes: ['error', 'single', { avoidEscape: true }]
  },
  settings: {
    react: {
      version: 'detect'
    }
  }
};
