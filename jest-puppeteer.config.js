module.exports = {
  launch: {
    dumpio: true,
    headless: true,
    product: 'chrome',
  },
  browserContext: 'incognito',
  server: [
    {
      command: 'rimraf ./e2e/output/* && npm run e2eServer',
      port: 3000,
    },
    {
      command: 'npm run dev',
      port: 8080,
    },
  ],
};
